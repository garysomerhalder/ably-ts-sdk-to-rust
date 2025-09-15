// 🔴 RED Phase: Ably encryption support with AES-128/256-CBC
// Following Ably protocol specification for message encryption
// Integration-First - real encryption using industry-standard AES

use crate::error::{AblyError, AblyResult};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::{Aes128, Aes256};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};

pub mod channel_cipher;

pub use channel_cipher::ChannelCipher;

/// AES cipher modes supported by Ably
type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;
type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Default algorithm for Ably encryption
const DEFAULT_ALGORITHM: &str = "aes";
/// Default key length in bits (256-bit)
const DEFAULT_KEY_LENGTH: usize = 256;
/// Default cipher mode
const DEFAULT_MODE: &str = "cbc";
/// AES block size in bytes
const BLOCK_SIZE: usize = 16;

/// Cipher parameters for encryption/decryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CipherParams {
    /// Encryption algorithm (e.g., "aes")
    pub algorithm: String,
    /// Key length in bits (128 or 256)
    pub key_length: usize,
    /// Cipher mode (e.g., "cbc")
    pub mode: String,
    /// Encryption key as bytes
    pub key: Vec<u8>,
}

impl CipherParams {
    /// Create cipher params from a key
    pub fn new(key: impl Into<Vec<u8>>) -> AblyResult<Self> {
        let key: Vec<u8> = key.into();
        let key_length = key.len() * 8; // Convert to bits
        
        // Validate key length
        if key_length != 128 && key_length != 256 {
            return Err(AblyError::invalid_request(format!(
                "Unsupported key length {} bits. Must be 128 or 256 bits (16 or 32 bytes)",
                key_length
            )));
        }
        
        Ok(Self {
            algorithm: DEFAULT_ALGORITHM.to_string(),
            key_length,
            mode: DEFAULT_MODE.to_string(),
            key,
        })
    }
    
    /// Create cipher params from base64-encoded key
    pub fn from_base64_key(base64_key: &str) -> AblyResult<Self> {
        let key = BASE64.decode(base64_key)
            .map_err(|e| AblyError::invalid_request(format!("Invalid base64 key: {}", e)))?;
        Self::new(key)
    }
    
    /// Generate a random encryption key
    pub fn generate_random_key(key_length: Option<usize>) -> AblyResult<Self> {
        let key_length = key_length.unwrap_or(DEFAULT_KEY_LENGTH);
        
        if key_length != 128 && key_length != 256 {
            return Err(AblyError::invalid_request(format!(
                "Unsupported key length {} bits. Must be 128 or 256 bits",
                key_length
            )));
        }
        
        let key_bytes = key_length / 8;
        let mut key = vec![0u8; key_bytes];
        rand::thread_rng().fill_bytes(&mut key);
        
        Self::new(key)
    }
    
    /// Get the key as base64-encoded string
    pub fn key_as_base64(&self) -> String {
        BASE64.encode(&self.key)
    }
    
    /// Get the algorithm string in Ably format (e.g., "aes-256-cbc")
    pub fn algorithm_string(&self) -> String {
        format!("{}-{}-{}", self.algorithm, self.key_length, self.mode)
    }
}

/// Main encryption interface for Ably protocol
pub struct AblyCrypto;

impl AblyCrypto {
    /// Encrypt plaintext data using the provided cipher parameters
    /// Returns IV + ciphertext as concatenated bytes
    pub fn encrypt(params: &CipherParams, plaintext: &[u8]) -> AblyResult<Vec<u8>> {
        // Generate random IV
        let mut iv = vec![0u8; BLOCK_SIZE];
        rand::thread_rng().fill_bytes(&mut iv);
        
        let ciphertext = match params.key_length {
            128 => Self::encrypt_aes128(&params.key, &iv, plaintext)?,
            256 => Self::encrypt_aes256(&params.key, &iv, plaintext)?,
            _ => return Err(AblyError::invalid_request(format!(
                "Unsupported key length: {} bits", params.key_length
            ))),
        };
        
        // Concatenate IV + ciphertext (Ably protocol format)
        let mut result = Vec::with_capacity(iv.len() + ciphertext.len());
        result.extend_from_slice(&iv);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    /// Decrypt ciphertext data using the provided cipher parameters
    /// Expects IV + ciphertext as concatenated bytes
    pub fn decrypt(params: &CipherParams, ciphertext: &[u8]) -> AblyResult<Vec<u8>> {
        if ciphertext.len() < BLOCK_SIZE {
            return Err(AblyError::invalid_request(
                "Ciphertext too short to contain IV".to_string()
            ));
        }
        
        // Extract IV and ciphertext body
        let (iv, ciphertext_body) = ciphertext.split_at(BLOCK_SIZE);
        
        match params.key_length {
            128 => Self::decrypt_aes128(&params.key, iv, ciphertext_body),
            256 => Self::decrypt_aes256(&params.key, iv, ciphertext_body),
            _ => Err(AblyError::invalid_request(format!(
                "Unsupported key length: {} bits", params.key_length
            ))),
        }
    }
    
    /// Encrypt using AES-128-CBC
    fn encrypt_aes128(key: &[u8], iv: &[u8], plaintext: &[u8]) -> AblyResult<Vec<u8>> {
        if key.len() != 16 {
            return Err(AblyError::invalid_request("AES-128 requires 16-byte key".to_string()));
        }
        
        let cipher = Aes128CbcEnc::new_from_slices(key, iv)
            .map_err(|e| AblyError::encryption(format!("Failed to create AES-128 cipher: {}", e)))?;
        
        // Calculate buffer size needed (plaintext + padding)
        let block_size = 16;
        let padded_len = ((plaintext.len() / block_size) + 1) * block_size;
        let mut buffer = vec![0u8; padded_len];
        buffer[..plaintext.len()].copy_from_slice(plaintext);
        
        let ciphertext = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .map_err(|e| AblyError::encryption(format!("AES-128 encryption failed: {}", e)))?;
        
        Ok(ciphertext.to_vec())
    }
    
    /// Decrypt using AES-128-CBC
    fn decrypt_aes128(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> AblyResult<Vec<u8>> {
        if key.len() != 16 {
            return Err(AblyError::invalid_request("AES-128 requires 16-byte key".to_string()));
        }
        
        let cipher = Aes128CbcDec::new_from_slices(key, iv)
            .map_err(|e| AblyError::encryption(format!("Failed to create AES-128 cipher: {}", e)))?;
        
        let mut buffer = ciphertext.to_vec();
        let plaintext = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| AblyError::encryption(format!("AES-128 decryption failed: {}", e)))?;
        
        Ok(plaintext.to_vec())
    }
    
    /// Encrypt using AES-256-CBC
    fn encrypt_aes256(key: &[u8], iv: &[u8], plaintext: &[u8]) -> AblyResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(AblyError::invalid_request("AES-256 requires 32-byte key".to_string()));
        }
        
        let cipher = Aes256CbcEnc::new_from_slices(key, iv)
            .map_err(|e| AblyError::encryption(format!("Failed to create AES-256 cipher: {}", e)))?;
        
        // Calculate buffer size needed (plaintext + padding)
        let block_size = 16;
        let padded_len = ((plaintext.len() / block_size) + 1) * block_size;
        let mut buffer = vec![0u8; padded_len];
        buffer[..plaintext.len()].copy_from_slice(plaintext);
        
        let ciphertext = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .map_err(|e| AblyError::encryption(format!("AES-256 encryption failed: {}", e)))?;
        
        Ok(ciphertext.to_vec())
    }
    
    /// Decrypt using AES-256-CBC
    fn decrypt_aes256(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> AblyResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(AblyError::invalid_request("AES-256 requires 32-byte key".to_string()));
        }
        
        let cipher = Aes256CbcDec::new_from_slices(key, iv)
            .map_err(|e| AblyError::encryption(format!("Failed to create AES-256 cipher: {}", e)))?;
        
        let mut buffer = ciphertext.to_vec();
        let plaintext = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| AblyError::encryption(format!("AES-256 decryption failed: {}", e)))?;
        
        Ok(plaintext.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cipher_params_creation() {
        // Test AES-128 key (16 bytes)
        let key_128 = vec![0u8; 16];
        let params = CipherParams::new(key_128).unwrap();
        assert_eq!(params.key_length, 128);
        assert_eq!(params.algorithm, "aes");
        assert_eq!(params.mode, "cbc");
        
        // Test AES-256 key (32 bytes)
        let key_256 = vec![0u8; 32];
        let params = CipherParams::new(key_256).unwrap();
        assert_eq!(params.key_length, 256);
        
        // Test invalid key length
        let key_invalid = vec![0u8; 24]; // 192 bits - not supported
        assert!(CipherParams::new(key_invalid).is_err());
    }
    
    #[test]
    fn test_base64_key_parsing() {
        // Valid base64 AES-128 key
        let base64_key = BASE64.encode(&vec![0u8; 16]);
        let params = CipherParams::from_base64_key(&base64_key).unwrap();
        assert_eq!(params.key_length, 128);
        
        // Invalid base64
        assert!(CipherParams::from_base64_key("invalid-base64!").is_err());
    }
    
    #[test]
    fn test_random_key_generation() {
        let params_128 = CipherParams::generate_random_key(Some(128)).unwrap();
        assert_eq!(params_128.key_length, 128);
        assert_eq!(params_128.key.len(), 16);
        
        let params_256 = CipherParams::generate_random_key(Some(256)).unwrap();
        assert_eq!(params_256.key_length, 256);
        assert_eq!(params_256.key.len(), 32);
        
        // Default should be 256-bit
        let params_default = CipherParams::generate_random_key(None).unwrap();
        assert_eq!(params_default.key_length, 256);
    }
    
    #[test]
    fn test_aes128_encryption_decryption() {
        let params = CipherParams::generate_random_key(Some(128)).unwrap();
        let plaintext = b"Hello, Ably encryption!";
        
        // Encrypt
        let ciphertext = AblyCrypto::encrypt(&params, plaintext).unwrap();
        assert!(ciphertext.len() > plaintext.len()); // Should be larger due to IV and padding
        
        // Decrypt
        let decrypted = AblyCrypto::decrypt(&params, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_aes256_encryption_decryption() {
        let params = CipherParams::generate_random_key(Some(256)).unwrap();
        let plaintext = b"Hello, Ably encryption with AES-256!";
        
        // Encrypt
        let ciphertext = AblyCrypto::encrypt(&params, plaintext).unwrap();
        assert!(ciphertext.len() > plaintext.len());
        
        // Decrypt
        let decrypted = AblyCrypto::decrypt(&params, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_algorithm_string_format() {
        let params_128 = CipherParams::generate_random_key(Some(128)).unwrap();
        assert_eq!(params_128.algorithm_string(), "aes-128-cbc");
        
        let params_256 = CipherParams::generate_random_key(Some(256)).unwrap();
        assert_eq!(params_256.algorithm_string(), "aes-256-cbc");
    }
    
    #[test]
    fn test_encryption_with_different_keys_produces_different_results() {
        let params1 = CipherParams::generate_random_key(Some(256)).unwrap();
        let params2 = CipherParams::generate_random_key(Some(256)).unwrap();
        let plaintext = b"Same plaintext";
        
        let ciphertext1 = AblyCrypto::encrypt(&params1, plaintext).unwrap();
        let ciphertext2 = AblyCrypto::encrypt(&params2, plaintext).unwrap();
        
        // Should be different due to different keys
        assert_ne!(ciphertext1, ciphertext2);
    }
    
    #[test]
    fn test_multiple_encryptions_produce_different_results() {
        let params = CipherParams::generate_random_key(Some(256)).unwrap();
        let plaintext = b"Same plaintext";
        
        let ciphertext1 = AblyCrypto::encrypt(&params, plaintext).unwrap();
        let ciphertext2 = AblyCrypto::encrypt(&params, plaintext).unwrap();
        
        // Should be different due to random IVs
        assert_ne!(ciphertext1, ciphertext2);
        
        // But both should decrypt to the same plaintext
        let decrypted1 = AblyCrypto::decrypt(&params, &ciphertext1).unwrap();
        let decrypted2 = AblyCrypto::decrypt(&params, &ciphertext2).unwrap();
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }
}