// 🟡 YELLOW Phase: Encryption implementation for Ably channels
// AES-128/256-CBC encryption following Ably specification

use crate::error::{AblyError, AblyResult};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// Cipher algorithms supported by Ably
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CipherAlgorithm {
    #[serde(rename = "aes")]
    Aes128,
    #[serde(rename = "aes-256")]
    Aes256,
}

impl fmt::Display for CipherAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CipherAlgorithm::Aes128 => write!(f, "aes-128"),
            CipherAlgorithm::Aes256 => write!(f, "aes-256"),
        }
    }
}

/// Cipher modes supported
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CipherMode {
    #[serde(rename = "cbc")]
    Cbc,
}

impl fmt::Display for CipherMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CipherMode::Cbc => write!(f, "cbc"),
        }
    }
}

/// Cipher parameters for channel encryption
#[derive(Debug, Clone)]
pub struct CipherParams {
    pub algorithm: CipherAlgorithm,
    pub mode: CipherMode,
    pub key: Vec<u8>,
    pub iv: Option<Vec<u8>>,
}

impl CipherParams {
    /// Create new cipher params with validation
    pub fn new(
        algorithm: CipherAlgorithm,
        mode: CipherMode,
        key: Vec<u8>,
        iv: Option<Vec<u8>>,
    ) -> AblyResult<Self> {
        // Validate key size
        let expected_key_size = match algorithm {
            CipherAlgorithm::Aes128 => 16,
            CipherAlgorithm::Aes256 => 32,
        };
        
        if key.len() != expected_key_size {
            return Err(AblyError::unexpected(format!(
                "Invalid key size: expected {} bytes, got {}",
                expected_key_size,
                key.len()
            )));
        }
        
        // Validate IV size if provided
        if let Some(ref iv_bytes) = iv {
            if iv_bytes.len() != 16 {
                return Err(AblyError::unexpected(format!(
                    "Invalid IV size: expected 16 bytes, got {}",
                    iv_bytes.len()
                )));
            }
        }
        
        Ok(Self {
            algorithm,
            mode,
            key,
            iv,
        })
    }
    
    /// Create AES-128-CBC cipher params
    pub fn aes128_cbc(key: Vec<u8>) -> AblyResult<Self> {
        Self::new(
            CipherAlgorithm::Aes128,
            CipherMode::Cbc,
            key,
            None,
        )
    }
    
    /// Create AES-256-CBC cipher params
    pub fn aes256_cbc(key: Vec<u8>) -> AblyResult<Self> {
        Self::new(
            CipherAlgorithm::Aes256,
            CipherMode::Cbc,
            key,
            None,
        )
    }
    
    /// Create cipher params from a key string
    pub fn from_key(key_str: &str) -> AblyResult<Self> {
        // Decode base64 key
        let key = base64::engine::general_purpose::STANDARD
            .decode(key_str)
            .map_err(|e| AblyError::unexpected(format!("Invalid base64 key: {}", e)))?;
        
        // Determine algorithm from key size
        let algorithm = match key.len() {
            16 => CipherAlgorithm::Aes128,
            32 => CipherAlgorithm::Aes256,
            _ => {
                return Err(AblyError::unexpected(format!(
                    "Invalid key size: {} bytes",
                    key.len()
                )))
            }
        };
        
        Ok(Self {
            algorithm,
            mode: CipherMode::Cbc,
            key,
            iv: None,
        })
    }
    
    /// Generate a random IV if not provided
    pub fn get_or_generate_iv(&self) -> Vec<u8> {
        if let Some(ref iv) = self.iv {
            iv.clone()
        } else {
            let mut iv = vec![0u8; 16];
            rand::thread_rng().fill(&mut iv[..]);
            iv
        }
    }
    
    /// Get encoding string for protocol
    pub fn encoding_string(&self) -> String {
        format!("cipher+{}-{}/base64", self.algorithm, self.mode)
    }
}

/// Channel cipher for encrypting/decrypting messages
#[derive(Clone)]
pub struct ChannelCipher {
    params: CipherParams,
}

impl ChannelCipher {
    /// Create a new channel cipher
    pub fn new(params: CipherParams) -> Self {
        Self { params }
    }
    
    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8]) -> AblyResult<EncryptedData> {
        let iv = self.params.get_or_generate_iv();
        
        let ciphertext = match self.params.algorithm {
            CipherAlgorithm::Aes128 => {
                self.encrypt_aes128_cbc(plaintext, &iv)?
            }
            CipherAlgorithm::Aes256 => {
                self.encrypt_aes256_cbc(plaintext, &iv)?
            }
        };
        
        Ok(EncryptedData {
            ciphertext,
            iv,
            encoding: self.params.encoding_string(),
        })
    }
    
    /// Decrypt data
    pub fn decrypt(&self, encrypted: &EncryptedData) -> AblyResult<Vec<u8>> {
        match self.params.algorithm {
            CipherAlgorithm::Aes128 => {
                self.decrypt_aes128_cbc(&encrypted.ciphertext, &encrypted.iv)
            }
            CipherAlgorithm::Aes256 => {
                self.decrypt_aes256_cbc(&encrypted.ciphertext, &encrypted.iv)
            }
        }
    }
    
    /// Encrypt with AES-128-CBC
    fn encrypt_aes128_cbc(&self, plaintext: &[u8], iv: &[u8]) -> AblyResult<Vec<u8>> {
        let cipher = Aes128CbcEnc::new_from_slices(&self.params.key, iv)
            .map_err(|e| AblyError::unexpected(format!("Cipher init failed: {}", e)))?;
        
        let mut buffer = plaintext.to_vec();
        let block_size = 16;
        let padding_needed = block_size - (buffer.len() % block_size);
        if padding_needed != block_size {
            buffer.resize(buffer.len() + padding_needed, padding_needed as u8);
        } else {
            buffer.extend(vec![block_size as u8; block_size]);
        }
        
        let ciphertext = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .map_err(|e| AblyError::unexpected(format!("Encryption failed: {}", e)))?
            .to_vec();
        
        Ok(ciphertext)
    }
    
    /// Decrypt with AES-128-CBC
    fn decrypt_aes128_cbc(&self, ciphertext: &[u8], iv: &[u8]) -> AblyResult<Vec<u8>> {
        let cipher = Aes128CbcDec::new_from_slices(&self.params.key, iv)
            .map_err(|e| AblyError::unexpected(format!("Cipher init failed: {}", e)))?;
        
        let mut buffer = ciphertext.to_vec();
        let plaintext = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| AblyError::unexpected(format!("Decryption failed: {}", e)))?
            .to_vec();
        
        Ok(plaintext)
    }
    
    /// Encrypt with AES-256-CBC
    fn encrypt_aes256_cbc(&self, plaintext: &[u8], iv: &[u8]) -> AblyResult<Vec<u8>> {
        let cipher = Aes256CbcEnc::new_from_slices(&self.params.key, iv)
            .map_err(|e| AblyError::unexpected(format!("Cipher init failed: {}", e)))?;
        
        let mut buffer = plaintext.to_vec();
        let block_size = 16;
        let padding_needed = block_size - (buffer.len() % block_size);
        if padding_needed != block_size {
            buffer.resize(buffer.len() + padding_needed, padding_needed as u8);
        } else {
            buffer.extend(vec![block_size as u8; block_size]);
        }
        
        let ciphertext = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .map_err(|e| AblyError::unexpected(format!("Encryption failed: {}", e)))?
            .to_vec();
        
        Ok(ciphertext)
    }
    
    /// Decrypt with AES-256-CBC
    fn decrypt_aes256_cbc(&self, ciphertext: &[u8], iv: &[u8]) -> AblyResult<Vec<u8>> {
        let cipher = Aes256CbcDec::new_from_slices(&self.params.key, iv)
            .map_err(|e| AblyError::unexpected(format!("Cipher init failed: {}", e)))?;
        
        let mut buffer = ciphertext.to_vec();
        let plaintext = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| AblyError::unexpected(format!("Decryption failed: {}", e)))?
            .to_vec();
        
        Ok(plaintext)
    }
}

/// Encrypted data with metadata
#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub iv: Vec<u8>,
    pub encoding: String,
}

impl EncryptedData {
    /// Convert to base64 for transmission
    pub fn to_base64(&self) -> String {
        // Combine IV and ciphertext
        let mut combined = self.iv.clone();
        combined.extend_from_slice(&self.ciphertext);
        
        base64::engine::general_purpose::STANDARD.encode(&combined)
    }
    
    /// Parse from base64
    pub fn from_base64(data: &str, encoding: String) -> AblyResult<Self> {
        let combined = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| AblyError::unexpected(format!("Invalid base64: {}", e)))?;
        
        if combined.len() < 16 {
            return Err(AblyError::unexpected("Data too short for IV + ciphertext"));
        }
        
        let (iv, ciphertext) = combined.split_at(16);
        
        Ok(Self {
            iv: iv.to_vec(),
            ciphertext: ciphertext.to_vec(),
            encoding,
        })
    }
}

/// Message crypto for encrypting/decrypting channel messages
#[derive(Clone)]
pub struct MessageCrypto {
    cipher: ChannelCipher,
}

impl MessageCrypto {
    /// Create new message crypto
    pub fn new(params: CipherParams) -> Self {
        Self {
            cipher: ChannelCipher::new(params),
        }
    }
    
    /// Encrypt a message
    pub fn encrypt_message(&self, message: &mut crate::protocol::messages::Message) -> AblyResult<()> {
        // Get message data as bytes
        let plaintext = match &message.data {
            Some(serde_json::Value::String(s)) => s.as_bytes().to_vec(),
            Some(v) => serde_json::to_vec(v)
                .map_err(|e| AblyError::unexpected(format!("Failed to serialize data: {}", e)))?,
            None => return Ok(()), // Nothing to encrypt
        };
        
        // Encrypt data
        let encrypted = self.cipher.encrypt(&plaintext)?;
        
        // Update message with encrypted data
        message.data = Some(serde_json::Value::String(encrypted.to_base64()));
        
        // Update encoding
        let encoding = if let Some(ref existing) = message.encoding {
            format!("{}/{}", encrypted.encoding, existing)
        } else {
            encrypted.encoding
        };
        message.encoding = Some(encoding);
        
        Ok(())
    }
    
    /// Decrypt a message
    pub fn decrypt_message(&self, message: &mut crate::protocol::messages::Message) -> AblyResult<()> {
        // Check if message is encrypted
        let encoding = match &message.encoding {
            Some(e) if e.contains("cipher") => e.clone(),
            _ => return Ok(()), // Not encrypted
        };
        
        // Get encrypted data
        let encrypted_str = match &message.data {
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => return Err(AblyError::unexpected("Invalid encrypted data format")),
        };
        
        // Parse encrypted data
        let encrypted = EncryptedData::from_base64(&encrypted_str, encoding.clone())?;
        
        // Decrypt
        let plaintext = self.cipher.decrypt(&encrypted)?;
        
        // Update message with decrypted data
        let data_str = String::from_utf8(plaintext)
            .map_err(|e| AblyError::unexpected(format!("Invalid UTF-8 in decrypted data: {}", e)))?;
        
        // Try to parse as JSON, otherwise keep as string
        message.data = Some(
            serde_json::from_str(&data_str)
                .unwrap_or_else(|_| serde_json::Value::String(data_str))
        );
        
        // Remove cipher from encoding
        let new_encoding = encoding
            .split('/')
            .filter(|part| !part.contains("cipher"))
            .collect::<Vec<_>>()
            .join("/");
        
        message.encoding = if new_encoding.is_empty() {
            None
        } else {
            Some(new_encoding)
        };
        
        Ok(())
    }
}

/// Generate a random key
pub fn generate_random_key(algorithm: CipherAlgorithm) -> Vec<u8> {
    let key_size = match algorithm {
        CipherAlgorithm::Aes128 => 16,
        CipherAlgorithm::Aes256 => 32,
    };
    
    let mut key = vec![0u8; key_size];
    rand::thread_rng().fill(&mut key[..]);
    key
}

/// Generate a random key as base64 string
pub fn generate_random_key_string(algorithm: CipherAlgorithm) -> String {
    let key = generate_random_key(algorithm);
    base64::engine::general_purpose::STANDARD.encode(&key)
}