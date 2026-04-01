use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of content, returned as a lowercase hex string.
pub fn sha256(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute short hash (first 8 hex characters of SHA-256).
pub fn short_hash(content: &[u8]) -> String {
    let full = sha256(content);
    full[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        // Known SHA-256 of empty input
        let hash = sha256(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hello() {
        let hash = sha256(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_deterministic() {
        let h1 = sha256(b"test data");
        let h2 = sha256(b"test data");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_sha256_different_inputs() {
        let h1 = sha256(b"hello");
        let h2 = sha256(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_sha256_length() {
        let hash = sha256(b"anything");
        assert_eq!(hash.len(), 64); // 256 bits = 64 hex chars
    }

    #[test]
    fn test_short_hash_length() {
        let hash = short_hash(b"anything");
        assert_eq!(hash.len(), 8);
    }

    #[test]
    fn test_short_hash_is_prefix() {
        let full = sha256(b"test");
        let short = short_hash(b"test");
        assert!(full.starts_with(&short));
    }

    #[test]
    fn test_short_hash_empty() {
        let hash = short_hash(b"");
        assert_eq!(hash, "e3b0c442");
    }
}
