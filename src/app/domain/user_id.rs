/// User ID domain type. Wraps ULID for non-sequential, sortable identifiers.
#[derive(Debug, Clone)]
pub struct UserId(ulid::Ulid);

impl UserId {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_new_id() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn parse_valid_ulid() {
        let original = UserId::new();
        let parsed = UserId::from_string(&original.as_str()).unwrap();
        assert_eq!(original.as_str(), parsed.as_str());
    }

    #[test]
    fn parse_invalid_ulid() {
        let result = UserId::from_string("invalid");
        assert!(result.is_err());
    }
}