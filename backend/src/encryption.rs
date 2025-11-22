// File: backend/src/encryption.rs
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use rand::RngCore;

/// Trait for pluggable encryption
pub trait Encryption: Send + Sync {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String>;
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String>;
}

/// No encryption (for development)
pub struct PlaintextEncryption;

impl Encryption for PlaintextEncryption {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
    
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
}

/// AES-256-GCM encryption (production)
pub struct AesGcmEncryption {
    cipher: Aes256Gcm,
}

impl AesGcmEncryption {
    /// Create new instance with random key (store this key securely!)
    pub fn new_random() -> Self {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }
    
    /// Create from existing key (load from config)
    pub fn from_key(key_bytes: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }
}

impl Encryption for AesGcmEncryption {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = self.cipher.encrypt(nonce, data)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        // Prepend nonce to ciphertext (needed for decryption)
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 12 {
            return Err("Data too short".to_string());
        }
        
        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt
        self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {:?}", e))
    }
}

// Usage example:
pub fn example_usage() {
    let enc: Box<dyn Encryption> = Box::new(PlaintextEncryption);
    
    let data = b"Hello, World!";
    let encrypted = enc.encrypt(data).unwrap();
    let decrypted = enc.decrypt(&encrypted).unwrap();
    
    assert_eq!(data.to_vec(), decrypted);
}