//! POST /api/projects/import — Create a new project from JSON.

use axum::{
    extract::State,
    response::Redirect,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use crate::app::{
    db,
    domain::OrganizationId,
    error::AppError,
    features::projects::import_export::{EXPORT_VERSION, ProjectExportEdge, ProjectExportNode, ProjectExportProject, ProjectExportSlot},
    session::ApiAuthenticatedSession,
    tenant,
    AppState,
};

/// Request body for import: optional team_id plus export payload.
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    #[serde(default)]
    pub team_id: Option<String>,
    pub version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub exported_at: Option<String>,
    pub project: ProjectExportProject,
    #[serde(default)]
    pub slots: Vec<ProjectExportSlot>,
    #[serde(default)]
    pub nodes: Vec<ProjectExportNode>,
    #[serde(default)]
    pub edges: Vec<ProjectExportEdge>,
}

/// Return indices into nodes so that for every edge (parent, child), parent's index is before child's.
/// Nodes not in any edge stay at the end. Assumes DAG; cycles may leave some nodes at the end.
fn topological_node_order(nodes: &[ProjectExportNode], edges: &[ProjectExportEdge]) -> Vec<usize> {
    let id_to_idx: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    let mut in_degree: Vec<usize> = vec![0; nodes.len()];
    for e in edges {
        if let Some(&c_idx) = id_to_idx.get(e.child_id.as_str()) {
            in_degree[c_idx] += 1;
        }
    }
    let mut queue: VecDeque<usize> = in_degree
        .iter()
        .enumerate()
        .filter(|(_, &d)| d == 0)
        .map(|(i, _)| i)
        .collect();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(i) = queue.pop_front() {
        order.push(i);
        let id = nodes[i].id.as_str();
        for e in edges {
            if e.parent_id.as_str() == id {
                if let Some(&c_idx) = id_to_idx.get(e.child_id.as_str()) {
                    in_degree[c_idx] = in_degree[c_idx].saturating_sub(1);
                    if in_degree[c_idx] == 0 {
                        queue.push_back(c_idx);
                    }
                }
            }
        }
    }
    for (i, _) in nodes.iter().enumerate() {
        if !order.contains(&i) {
            order.push(i);
        }
    }
    order
}

/// POST /api/projects/import — Import project from JSON; redirect to new project.
pub async fn import_project(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Json(body): Json<ImportRequest>,
) -> Result<Redirect, AppError> {
    // Validation first: version and project title
    if body.version != EXPORT_VERSION {
        return Err(AppError::Validation(format!(
            "Unsupported export version {}; expected {}",
            body.version, EXPORT_VERSION
        )));
    }
    let title = body.project.title.trim();
    if title.is_empty() {
        return Err(AppError::Validation("Project title is required".to_string()));
    }

    // Org membership
    tenant::require_org_member(&state.db, &session.user_id, &session.organization_id).await?;

    // Resolve team: body.team_id or default for org
    let org_id = OrganizationId::from_string(&session.organization_id)
        .map_err(|_| AppError::Validation("Invalid organization".to_string()))?;
    let team_id = if let Some(ref id) = body.team_id {
        if id.is_empty() {
            db::teams::find_default_for_org(&state.db, &org_id)
                .await?
                .map(|t| t.id)
                .ok_or_else(|| AppError::Validation("No team found for organization".to_string()))?
        } else {
            let team = db::teams::find_by_id(&state.db, id).await?;
            let team = team.ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;
            if team.organization_id != session.organization_id {
                return Err(AppError::NotFound("Team not found".to_string()));
            }
            team.id
        }
    } else {
        db::teams::find_default_for_org(&state.db, &org_id)
            .await?
            .map(|t| t.id)
            .ok_or_else(|| AppError::Validation("No team found for organization".to_string()))?
    };

    let mut tx = state.db.begin().await?;

    let new_project_id = ulid::Ulid::new().to_string();
    let new_project = db::projects::NewProject {
        id: new_project_id.clone(),
        title: title.to_string(),
        user_id: session.user_id.clone(),
        organization_id: session.organization_id.clone(),
        team_id: team_id.clone(),
    };
    db::projects::insert(&mut *tx, &new_project).await?;

    let mut slot_map: HashMap<String, String> = HashMap::new();
    for s in &body.slots {
        let new_id = ulid::Ulid::new().to_string();
        slot_map.insert(s.id.clone(), new_id.clone());
        let new_slot = db::project_slots::NewProjectSlot {
            id: new_id,
            project_id: new_project_id.clone(),
            name: s.name.clone(),
            sort_order: s.sort_order,
        };
        db::project_slots::insert(&mut *tx, &new_slot).await?;
    }

    // Map old node id -> new node id; insert nodes in topological order (parents before children)
    let mut node_map: HashMap<String, String> = HashMap::new();
    let node_order = topological_node_order(&body.nodes, &body.edges);
    for i in node_order {
        let n = &body.nodes[i];
        let new_id = ulid::Ulid::new().to_string();
        node_map.insert(n.id.clone(), new_id.clone());

        let slot_id = n.slot_id.as_ref().and_then(|id| slot_map.get(id).cloned());
        let parent_id = n.parent_id.as_ref().and_then(|id| node_map.get(id).cloned());

        let new_node = db::nodes::NewNode {
            id: new_id,
            project_id: new_project_id.clone(),
            node_type_id: n.node_type_id.clone(),
            status_id: n.status_id.clone(),
            title: n.title.clone(),
            description: n.description.clone(),
            estimated_minutes: n.estimated_minutes,
            slot_id,
            parent_id,
            assigned_user_id: None,
        };
        db::nodes::insert(&mut *tx, &new_node).await?;
    }

    // Insert edges (both endpoints must be in node_map)
    for e in &body.edges {
        if let (Some(new_parent), Some(new_child)) =
            (node_map.get(&e.parent_id), node_map.get(&e.child_id))
        {
            let new_edge = db::node_edges::NewNodeEdge {
                parent_id: new_parent.clone(),
                child_id: new_child.clone(),
            };
            let _ = db::node_edges::insert_if_not_exists(&mut *tx, &new_edge).await;
        }
    }

    tx.commit().await?;

    Ok(Redirect::to(&format!("/app/projects/{}", new_project_id)))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/import", post(import_project))
}
