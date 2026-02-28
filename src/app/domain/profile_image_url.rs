use crate::app::domain::validation_helpers;

/// Profile image URL domain type. Once constructed, guaranteed to be valid: trimmed, HTTPS, image extension, max length.
#[derive(Debug, Clone)]
pub struct ProfileImageUrl(String);

impl ProfileImageUrl {
    const MAX_URL_LEN: usize = 2048;

    /// Create a new ProfileImageUrl from a string. Validates HTTPS, length, and image extension.
    /// Trims whitespace. Returns an error if validation fails.
    /// Empty or whitespace-only input is not valid for construction; use `Option<ProfileImageUrl>` to represent "no URL".
    pub fn new(url: impl AsRef<str>) -> Result<Self, &'static str> {
        let t = url.as_ref().trim();
        if t.is_empty() {
            return Err("Profile image URL cannot be empty.");
        }
        validation_helpers::check_https_scheme(t)?;
        Self::check_length(t)?;
        validation_helpers::check_image_extension(t)?;
        Ok(Self(t.to_string()))
    }

    fn check_length(t: &str) -> Result<(), &'static str> {
        if t.len() > Self::MAX_URL_LEN {
            Err("Profile image URL is too long.")
        } else {
            Ok(())
        }
    }

    /// Get the URL as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_https_image_url() {
        let url = ProfileImageUrl::new("https://example.com/photo.jpg").unwrap();
        assert_eq!(url.as_str(), "https://example.com/photo.jpg");
    }

    #[test]
    fn trims_whitespace() {
        let url = ProfileImageUrl::new("  https://example.com/avatar.png  ").unwrap();
        assert_eq!(url.as_str(), "https://example.com/avatar.png");
    }

    #[test]
    fn rejects_http() {
        let result = ProfileImageUrl::new("http://example.com/photo.jpg");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_image_extension() {
        let result = ProfileImageUrl::new("https://example.com/document.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty() {
        let result = ProfileImageUrl::new("");
        assert!(result.is_err());
    }

    #[test]
    fn accepts_webp_and_gif() {
        assert!(ProfileImageUrl::new("https://example.com/a.webp").is_ok());
        assert!(ProfileImageUrl::new("https://example.com/b.gif").is_ok());
    }
}
