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

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

// --- Password hashing (bcrypt) ---

pub fn hash_password(plain: &str) -> Result<String, String> {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())
}

pub fn verify_password(plain: &str, hash: &str) -> bool {
    bcrypt::verify(plain, hash).unwrap_or(false)
}

pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    if password.len() > 128 {
        return Err("Password must be at most 128 characters".to_string());
    }
    Ok(())
}

pub fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 2 {
        return Err("Username must be at least 2 characters".to_string());
    }
    if username.len() > 32 {
        return Err("Username must be at most 32 characters".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Username can only contain letters, numbers, hyphens, and underscores".to_string());
    }
    Ok(())
}

// --- JWT ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_jwt(user_id: &str, role: &str, secret: &str) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = JwtClaims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp: now + 86400,
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<JwtClaims, String> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(token_data.claims)
}

// --- Refresh Tokens ---

pub fn create_refresh_token() -> String {
    format!("rt_{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

pub fn create_refresh_jwt(user_id: &str, secret: &str) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = RefreshClaims {
        sub: user_id.to_string(),
        exp: now + 604800, // 7 days
        iat: now,
        jti: uuid::Uuid::new_v4().to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn verify_refresh_jwt(token: &str, secret: &str) -> Result<RefreshClaims, String> {
    let token_data = decode::<RefreshClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(token_data.claims)
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

    #[test]
    fn test_hash_and_verify_password() {
        let hash = hash_password("my-password").unwrap();
        assert!(verify_password("my-password", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("short").is_err());
        assert!(validate_password("").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("test_user").is_ok());
        assert!(validate_username("a").is_err());
        assert!(validate_username("invalid user!").is_err());
        assert!(validate_username("").is_err());
    }

    #[test]
    fn test_create_and_verify_jwt() {
        let secret = "test-secret";
        let token = create_jwt("user-1", "admin", secret).unwrap();
        let claims = verify_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn test_verify_jwt_wrong_secret() {
        let token = create_jwt("user-1", "admin", "secret-1").unwrap();
        assert!(verify_jwt(&token, "secret-2").is_err());
    }

    #[test]
    fn test_verify_jwt_expired() {
        let claims = JwtClaims {
            sub: "user-1".to_string(),
            role: "admin".to_string(),
            exp: 0,
            iat: 0,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_bytes()),
        )
        .unwrap();
        assert!(verify_jwt(&token, "secret").is_err());
    }

    #[test]
    fn test_create_refresh_token_format() {
        let rt = create_refresh_token();
        assert!(rt.starts_with("rt_"));
        assert_eq!(rt.len(), 35); // "rt_" + 32 hex chars
    }

    #[test]
    fn test_create_and_verify_refresh_jwt() {
        let secret = "test-secret";
        let token = create_refresh_jwt("user-1", secret).unwrap();
        let claims = verify_refresh_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, "user-1");
    }

    #[test]
    fn test_verify_refresh_jwt_wrong_secret() {
        let token = create_refresh_jwt("user-1", "secret-1").unwrap();
        assert!(verify_refresh_jwt(&token, "secret-2").is_err());
    }

    #[test]
    fn test_verify_refresh_jwt_expired() {
        let claims = RefreshClaims {
            sub: "user-1".to_string(),
            exp: 0,
            iat: 0,
            jti: "test-jti".to_string(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_bytes()),
        )
        .unwrap();
        assert!(verify_refresh_jwt(&token, "secret").is_err());
    }
}
