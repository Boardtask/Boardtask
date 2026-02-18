/// Organization ID domain type. Wraps ULID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OrganizationId(ulid::Ulid);

impl OrganizationId {
    /// Generate a new random ULID.
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }

    /// Get as string for storage/display.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Parse from string.
    pub fn from_string(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(ulid::Ulid::from_string(s)?))
    }

    /// Get the inner ULID.
    pub fn inner(&self) -> &ulid::Ulid {
        &self.0
    }
}

impl std::fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
