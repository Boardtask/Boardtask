/// Allowed image file extensions for image URLs (lowercase).
pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];

/// Check that a trimmed, non-empty URL uses the HTTPS scheme.
///
/// This does not trim or handle empties itself; callers should decide whether
/// empty/whitespace-only values are allowed and pass a trimmed string.
pub fn check_https_scheme(t: &str) -> Result<(), &'static str> {
    if t.starts_with("https://") {
        Ok(())
    } else {
        Err("URL must use HTTPS.")
    }
}

/// Check that a trimmed, non-empty URL points to a common image extension.
///
/// This strips query and fragment, then inspects the last path segment's
/// extension and checks it against `ALLOWED_IMAGE_EXTENSIONS`.
pub fn check_image_extension(t: &str) -> Result<(), &'static str> {
    let path = t.split('?').next().unwrap_or(t).split('#').next().unwrap_or(t);
    let filename = path.rsplit('/').next().unwrap_or(path);
    let ext = filename.rsplit('.').next().unwrap_or("");
    let ext_lower = ext.to_lowercase();
    if ext.is_empty() || !ALLOWED_IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
        Err("URL must point to an image.")
    } else {
        Ok(())
    }
}
