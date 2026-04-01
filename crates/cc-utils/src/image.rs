/// Supported image extensions.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];

/// Maximum image file size: 20 MB.
const MAX_IMAGE_SIZE: u64 = 20 * 1024 * 1024;

/// Check if a file extension is a supported image format.
/// The extension is matched case-insensitively and without a leading dot.
pub fn is_image_extension(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    // Strip leading dot if present
    let ext_clean = lower.strip_prefix('.').unwrap_or(&lower);
    IMAGE_EXTENSIONS.contains(&ext_clean)
}

/// Check if file size is within image limits (20MB).
pub fn is_within_size_limit(file_size: u64) -> bool {
    file_size <= MAX_IMAGE_SIZE
}

/// Get media type from file extension.
/// Returns `None` if the extension is not a recognized image format.
pub fn media_type_from_extension(ext: &str) -> Option<&'static str> {
    let lower = ext.to_lowercase();
    let ext_clean = lower.strip_prefix('.').unwrap_or(&lower);
    match ext_clean {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_extension_supported() {
        assert!(is_image_extension("png"));
        assert!(is_image_extension("jpg"));
        assert!(is_image_extension("jpeg"));
        assert!(is_image_extension("gif"));
        assert!(is_image_extension("webp"));
        assert!(is_image_extension("bmp"));
        assert!(is_image_extension("svg"));
    }

    #[test]
    fn test_is_image_extension_case_insensitive() {
        assert!(is_image_extension("PNG"));
        assert!(is_image_extension("Jpg"));
        assert!(is_image_extension("JPEG"));
        assert!(is_image_extension("GIF"));
    }

    #[test]
    fn test_is_image_extension_with_dot() {
        assert!(is_image_extension(".png"));
        assert!(is_image_extension(".jpg"));
    }

    #[test]
    fn test_is_image_extension_unsupported() {
        assert!(!is_image_extension("txt"));
        assert!(!is_image_extension("pdf"));
        assert!(!is_image_extension("rs"));
        assert!(!is_image_extension("mp4"));
        assert!(!is_image_extension(""));
    }

    #[test]
    fn test_is_within_size_limit() {
        assert!(is_within_size_limit(0));
        assert!(is_within_size_limit(1024));
        assert!(is_within_size_limit(20 * 1024 * 1024)); // exactly 20MB
        assert!(!is_within_size_limit(20 * 1024 * 1024 + 1)); // just over
    }

    #[test]
    fn test_media_type_from_extension() {
        assert_eq!(media_type_from_extension("png"), Some("image/png"));
        assert_eq!(media_type_from_extension("jpg"), Some("image/jpeg"));
        assert_eq!(media_type_from_extension("jpeg"), Some("image/jpeg"));
        assert_eq!(media_type_from_extension("gif"), Some("image/gif"));
        assert_eq!(media_type_from_extension("webp"), Some("image/webp"));
        assert_eq!(media_type_from_extension("bmp"), Some("image/bmp"));
        assert_eq!(media_type_from_extension("svg"), Some("image/svg+xml"));
    }

    #[test]
    fn test_media_type_from_extension_case_insensitive() {
        assert_eq!(media_type_from_extension("PNG"), Some("image/png"));
        assert_eq!(media_type_from_extension("JPG"), Some("image/jpeg"));
    }

    #[test]
    fn test_media_type_from_extension_with_dot() {
        assert_eq!(media_type_from_extension(".png"), Some("image/png"));
    }

    #[test]
    fn test_media_type_from_extension_unknown() {
        assert_eq!(media_type_from_extension("txt"), None);
        assert_eq!(media_type_from_extension("mp4"), None);
        assert_eq!(media_type_from_extension(""), None);
    }

    #[test]
    fn test_image_extensions_constant() {
        assert_eq!(IMAGE_EXTENSIONS.len(), 7);
        assert!(IMAGE_EXTENSIONS.contains(&"png"));
        assert!(IMAGE_EXTENSIONS.contains(&"svg"));
    }
}
