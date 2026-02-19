//! Progress and blocked counts for project show (computed in code from nodes + edges).

use std::collections::HashMap;

use crate::app::db::{node_edges, nodes, task_statuses};

/// Counts nodes that are blocked (dependent on an incomplete parent).
/// Returns (blocked_count, blocked_todo_count, blocked_in_progress_count).
pub fn count_blocked(
    nodes: &[nodes::Node],
    edges: &[node_edges::NodeEdge],
) -> (i64, i64, i64) {
    let parent_ids_by_child: HashMap<&str, Vec<&str>> = {
        let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in edges {
            m.entry(e.child_id.as_str())
                .or_default()
                .push(e.parent_id.as_str());
        }
        m
    };

    let status_by_id: HashMap<&str, &str> = nodes
        .iter()
        .map(|n| (n.id.as_str(), n.status_id.as_str()))
        .collect();

    let is_root = |id: &str| !edges.iter().any(|e| e.child_id.as_str() == id);
    let is_done = |id: &str| {
        status_by_id
            .get(id)
            .map_or(false, |sid| *sid == task_statuses::DONE_STATUS_ID)
    };
    let has_blocking_parent = |id: &str| {
        parent_ids_by_child.get(id).map_or(false, |pids| {
            pids.iter()
                .any(|pid| !is_root(pid) && !is_done(pid))
        })
    };
    let is_blocked = |id: &str| !is_root(id) && !is_done(id) && has_blocking_parent(id);

    let mut blocked_count: i64 = 0;
    let mut blocked_todo_count: i64 = 0;
    let mut blocked_in_progress_count: i64 = 0;

    for n in nodes {
        if !is_blocked(n.id.as_str()) {
            continue;
        }
        blocked_count += 1;
        match n.status_id.as_str() {
            s if s == task_statuses::TODO_STATUS_ID => blocked_todo_count += 1,
            s if s == task_statuses::IN_PROGRESS_STATUS_ID => blocked_in_progress_count += 1,
            _ => {}
        }
    }

    (blocked_count, blocked_todo_count, blocked_in_progress_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::db::{node_edges, nodes, task_statuses};

    fn node(id: &str, status_id: &str) -> nodes::Node {
        nodes::Node {
            id: id.to_string(),
            project_id: "project".to_string(),
            node_type_id: "type".to_string(),
            status_id: status_id.to_string(),
            title: "".to_string(),
            description: None,
            created_at: 0,
            updated_at: None,
            estimated_minutes: None,
            slot_id: None,
        }
    }

    fn edge(parent_id: &str, child_id: &str) -> node_edges::NodeEdge {
        node_edges::NodeEdge {
            parent_id: parent_id.to_string(),
            child_id: child_id.to_string(),
            created_at: 0,
        }
    }

    #[test]
    fn empty_graph_has_no_blocked() {
        let nodes: Vec<nodes::Node> = vec![];
        let edges: Vec<node_edges::NodeEdge> = vec![];
        let (blocked, blocked_todo, blocked_in_progress) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 0);
        assert_eq!(blocked_todo, 0);
        assert_eq!(blocked_in_progress, 0);
    }

    #[test]
    fn single_node_no_edges_is_not_blocked() {
        let nodes = vec![node("a", task_statuses::TODO_STATUS_ID)];
        let edges: Vec<node_edges::NodeEdge> = vec![];
        let (blocked, blocked_todo, blocked_in_progress) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 0);
        assert_eq!(blocked_todo, 0);
        assert_eq!(blocked_in_progress, 0);
    }

    #[test]
    fn child_blocked_when_non_root_parent_not_done() {
        // A -> B -> C: only a parent that is itself not a root and not done blocks.
        // So C is blocked (B is non-root and todo); B is not blocked (A is root).
        let nodes = vec![
            node("a", task_statuses::TODO_STATUS_ID),
            node("b", task_statuses::TODO_STATUS_ID),
            node("c", task_statuses::TODO_STATUS_ID),
        ];
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let (blocked, blocked_todo, blocked_in_progress) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 1);
        assert_eq!(blocked_todo, 1);
        assert_eq!(blocked_in_progress, 0);
    }

    #[test]
    fn child_not_blocked_when_parent_done() {
        // A (done) -> B (todo): B is not blocked
        let nodes = vec![
            node("a", task_statuses::DONE_STATUS_ID),
            node("b", task_statuses::TODO_STATUS_ID),
        ];
        let edges = vec![edge("a", "b")];
        let (blocked, blocked_todo, blocked_in_progress) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 0);
        assert_eq!(blocked_todo, 0);
        assert_eq!(blocked_in_progress, 0);
    }

    #[test]
    fn blocked_in_progress_counted_correctly() {
        // A -> B (in progress) -> C (in progress): C is blocked (B is non-root and not done)
        let nodes = vec![
            node("a", task_statuses::TODO_STATUS_ID),
            node("b", task_statuses::IN_PROGRESS_STATUS_ID),
            node("c", task_statuses::IN_PROGRESS_STATUS_ID),
        ];
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let (blocked, blocked_todo, blocked_in_progress) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 1);
        assert_eq!(blocked_todo, 0);
        assert_eq!(blocked_in_progress, 1);
    }

    #[test]
    fn chain_blocks_only_until_done() {
        // A -> B -> C: blocking parent = non-root and not done. So only C can be blocked (by B).
        let nodes = vec![
            node("a", task_statuses::TODO_STATUS_ID),
            node("b", task_statuses::TODO_STATUS_ID),
            node("c", task_statuses::TODO_STATUS_ID),
        ];
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let (blocked, _, _) = count_blocked(&nodes, &edges);
        assert_eq!(blocked, 1, "only C is blocked (B is non-root and todo)");

        let nodes_done_b = vec![
            node("a", task_statuses::TODO_STATUS_ID),
            node("b", task_statuses::DONE_STATUS_ID),
            node("c", task_statuses::TODO_STATUS_ID),
        ];
        let (blocked2, _, _) = count_blocked(&nodes_done_b, &edges);
        assert_eq!(blocked2, 0, "C not blocked when parent B is done");
    }
}
