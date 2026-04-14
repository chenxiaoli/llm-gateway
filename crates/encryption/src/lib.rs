use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;

const NONCE_SIZE: usize = 12;

#[derive(Debug)]
pub enum EncryptionError {
    EncryptFailed(String),
    DecryptFailed(String),
    InvalidKeyLength(usize),
    InvalidFormat,
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::EncryptFailed(msg) => write!(f, "encryption failed: {}", msg),
            EncryptionError::DecryptFailed(msg) => write!(f, "decryption failed: {}", msg),
            EncryptionError::InvalidKeyLength(len) => write!(f, "invalid key length: expected 32 bytes, got {}", len),
            EncryptionError::InvalidFormat => write!(f, "invalid format: expected base64 encoded nonce(12) || ciphertext"),
        }
    }
}

impl std::error::Error for EncryptionError {}

/// Encrypts plaintext using AES-256-GCM with a random nonce.
/// Returns base64(nonce || ciphertext).
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength(key.len()));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptionError::EncryptFailed(e.to_string()))?;

    let mut rng = rand::thread_rng();
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rng.fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::EncryptFailed(e.to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);

    Ok(BASE64.encode(result))
}

/// Decrypts ciphertext using AES-256-GCM.
/// Input is base64(nonce || ciphertext).
pub fn decrypt(ciphertext: &str, key: &[u8; 32]) -> Result<String, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength(key.len()));
    }

    let data = BASE64
        .decode(ciphertext)
        .map_err(|_| EncryptionError::InvalidFormat)?;

    if data.len() < NONCE_SIZE {
        return Err(EncryptionError::InvalidFormat);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptionError::DecryptFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
    let encrypted_data = &data[NONCE_SIZE..];

    let plaintext = cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| EncryptionError::DecryptFailed(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| EncryptionError::DecryptFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0u8; 32];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(std::str::from_utf8(plaintext).unwrap(), &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key).unwrap();

        assert_eq!(plaintext, decrypted.as_bytes());
    }

    #[test]
    fn test_different_ciphertexts_for_same_plaintext() {
        let key = [0u8; 32];
        let plaintext = b"Same message";

        let ciphertext1 = encrypt(std::str::from_utf8(plaintext).unwrap(), &key).unwrap();
        let ciphertext2 = encrypt(std::str::from_utf8(plaintext).unwrap(), &key).unwrap();

        // Ciphertexts should be different due to random nonce
        assert_ne!(ciphertext1, ciphertext2);

        // Both should decrypt to the same plaintext
        assert_eq!(decrypt(&ciphertext1, &key).unwrap().as_bytes(), plaintext);
        assert_eq!(decrypt(&ciphertext2, &key).unwrap().as_bytes(), plaintext);
    }
}
