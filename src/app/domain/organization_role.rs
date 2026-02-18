use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Organization role enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "lowercase")] // Serialize as lowercase string
#[strum(serialize_all = "lowercase")] // Display/FromStr as lowercase string
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
    Viewer,
}
