use crate::error::ProxyError;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

pub struct KeyStore {
    key: LessSafeKey,
}

impl KeyStore {
    pub fn new(master_key: &[u8; 32]) -> Self {
        let unbound_key =
            UnboundKey::new(&AES_256_GCM, master_key).expect("Failed to create encryption key");
        Self {
            key: LessSafeKey::new(unbound_key),
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), ProxyError> {
        let rng = SystemRandom::new();
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|_| ProxyError::KeyManagement("Failed to generate nonce".into()))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = plaintext.as_bytes().to_vec();
        self.key
            .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| ProxyError::KeyManagement("Encryption failed".into()))?;

        Ok((in_out, nonce_bytes.to_vec()))
    }

    pub fn decrypt(&self, encrypted: &[u8], nonce_bytes: &[u8]) -> Result<String, ProxyError> {
        let nonce_arr: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| ProxyError::KeyManagement("Invalid nonce".into()))?;
        let nonce = Nonce::assume_unique_for_key(nonce_arr);

        let mut in_out = encrypted.to_vec();
        let plaintext = self
            .key
            .open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| ProxyError::KeyManagement("Decryption failed".into()))?;

        String::from_utf8(plaintext.to_vec())
            .map_err(|e| ProxyError::KeyManagement(format!("Invalid UTF-8: {}", e)))
    }

    pub fn derive_key() -> [u8; 32] {
        use std::hash::{Hash, Hasher};
        let machine_id = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "default-machine-id".to_string());

        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        machine_id.hash(&mut hasher);
        let seed = hasher.finish();

        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = ((seed >> (i % 8 * 8)) & 0xFF) as u8;
            key[i] ^= b"AI_PROXY_SALT_2026"[i.min(17)];
        }
        key
    }
}
