//! Tenant isolation enforcement.
//!
//! **Rule**: Never trust session org. Validate membership on every read/write.

use crate::app::{db, domain::OrganizationId, error::AppError};
use crate::app::domain::{OrganizationRole, UserId};

/// Validates that the user is a member of the organisation. Returns the member's role.
/// Use this on every write and when scoping reads by org.
///
/// Returns `NotFound` (not `Auth`) to avoid leaking whether the org exists.
pub async fn require_org_member(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    org_id: &str,
) -> Result<OrganizationRole, AppError> {
    let user_id = UserId::from_string(user_id).map_err(|_| AppError::NotFound("Not found".to_string()))?;
    let org_id = OrganizationId::from_string(org_id).map_err(|_| AppError::NotFound("Not found".to_string()))?;

    db::organizations::find_member_role(pool, &org_id, &user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Not found".to_string()))
}
