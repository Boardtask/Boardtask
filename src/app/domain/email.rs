use validator::ValidationError;

/// Email domain type. Once constructed, guaranteed to be valid, trimmed, and lowercase.
#[derive(Debug, Clone)]
pub struct Email(String);

impl Email {
    /// Create a new Email from a string. Validates format, trims whitespace, and converts to lowercase.
    /// Returns an error if the email format is invalid.
    pub fn new(email: String) -> Result<Self, ValidationError> {
        let normalized = email.trim().to_lowercase();

        // Maximum email length per RFC 5321
        if normalized.len() > 254 {
            let mut error = ValidationError::new("email_too_long");
            error.message = Some("Email address is too long".into());
            return Err(error);
        }

        // Basic email validation - contains @ and has domain
        if normalized.contains('@') && normalized.split('@').nth(1).map_or(false, |domain| domain.contains('.')) {
            Ok(Self(normalized))
        } else {
            let mut error = ValidationError::new("invalid_email");
            error.message = Some("Invalid email address format".into());
            Err(error)
        }
    }

    /// Get the email as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email() {
        let email = Email::new("test@example.com".to_string()).unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn email_trimmed_and_lowercased() {
        let email = Email::new("  TeSt@ExAmPlE.CoM  ".to_string()).unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn invalid_email_format() {
        let result = Email::new("notanemail".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn email_too_long() {
        let long_email = "a".repeat(250) + "@example.com";
        let result = Email::new(long_email);
        assert!(result.is_err());
    }
}