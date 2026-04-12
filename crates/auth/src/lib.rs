use sha2::{Digest, Sha256};

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_api_key(key: &str, hash: &str) -> bool {
    hash_api_key(key) == hash
}

pub fn generate_api_key() -> String {
    format!("sk-{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_api_key("test-key");
        let h2 = hash_api_key("test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_keys_differ() {
        let h1 = hash_api_key("key-a");
        let h2 = hash_api_key("key-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_correct_key() {
        let hash = hash_api_key("my-secret-key");
        assert!(verify_api_key("my-secret-key", &hash));
    }

    #[test]
    fn test_verify_wrong_key() {
        let hash = hash_api_key("my-secret-key");
        assert!(!verify_api_key("wrong-key", &hash));
    }

    #[test]
    fn test_generate_key_format() {
        let key = generate_api_key();
        assert!(key.starts_with("sk-"));
        assert_eq!(key.len(), 35); // "sk-" + 32 hex chars
    }

    #[test]
    fn test_generate_keys_unique() {
        let k1 = generate_api_key();
        let k2 = generate_api_key();
        assert_ne!(k1, k2);
    }
}
