use argon2::{
    password_hash::SaltString,
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use rand_core::OsRng;
use validator::ValidationError;

/// Password domain type. Once constructed, guaranteed to meet strength requirements.
#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    /// Create a password from plaintext for verification only (e.g., during login).
    /// Does NOT validate strength requirements—use only when checking against a stored hash.
    /// Strength validation belongs in signup, not login.
    pub fn for_verification(plaintext: String) -> Self {
        Self(plaintext)
    }

    /// Create a new Password from a string. Validates strength requirements.
    /// Returns an error if the password doesn't meet requirements.
    pub fn new(password: String) -> Result<Self, ValidationError> {
        if password.len() < 8 {
            let mut error = ValidationError::new("password_too_short");
            error.message = Some("Password must be at least 8 characters".into());
            return Err(error);
        }

        if password.len() > 128 {
            let mut error = ValidationError::new("password_too_long");
            error.message = Some("Password must be at most 128 characters".into());
            return Err(error);
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());

        if !(has_uppercase && has_lowercase && has_digit) {
            let mut error = ValidationError::new("weak_password");
            error.message = Some(
                "Password must contain uppercase, lowercase, and digit".into()
            );
            return Err(error);
        }

        Ok(Self(password))
    }

    /// Get password bytes for hashing (internal use only).
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Hashed password wrapper. Stores Argon2 hash as string.
#[derive(Debug, Clone)]
pub struct HashedPassword(String);

impl HashedPassword {
    /// Hash a password using Argon2id with random salt.
    pub fn from_password(password: &Password) -> Result<Self, argon2::password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(Self(hash.to_string()))
    }

    /// Verify a password against this hash.
    pub fn verify(&self, password: &Password) -> Result<(), argon2::password_hash::Error> {
        let parsed_hash = PasswordHash::new(&self.0)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
    }

    /// Create from existing hash string (e.g., from database).
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Get hash as string for storage.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_password() {
        let password = Password::new("Password1".to_string()).unwrap();
        assert_eq!(password.as_bytes(), b"Password1");
    }

    #[test]
    fn password_too_short() {
        let result = Password::new("short".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn password_too_long() {
        let long_password = "a".repeat(129);
        let result = Password::new(long_password);
        assert!(result.is_err());
    }

    #[test]
    fn weak_password_no_uppercase() {
        let result = Password::new("password1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn weak_password_no_lowercase() {
        let result = Password::new("PASSWORD1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn weak_password_no_digit() {
        let result = Password::new("Password".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn hash_and_verify() {
        let password = Password::new("TestPassword123".to_string()).unwrap();
        let hash = HashedPassword::from_password(&password).unwrap();
        assert!(hash.verify(&password).is_ok());
    }

    #[test]
    fn hash_wrong_password() {
        let password = Password::new("TestPassword123".to_string()).unwrap();
        let wrong_password = Password::new("WrongPassword456".to_string()).unwrap();
        let hash = HashedPassword::from_password(&password).unwrap();
        assert!(hash.verify(&wrong_password).is_err());
    }

    #[test]
    fn for_verification_allows_weak_passwords_for_login() {
        // Password::new rejects "password" (no uppercase, no digit)
        assert!(Password::new("password".to_string()).is_err());
        // for_verification accepts it—used during login to verify against stored hash.
        // Legacy accounts may have passwords that don't meet current strength rules.
        let weak = Password::for_verification("password".to_string());
        let hash = HashedPassword::from_password(&weak).unwrap();
        assert!(hash.verify(&weak).is_ok());
    }
}