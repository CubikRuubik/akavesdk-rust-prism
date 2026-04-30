#[cfg(not(target_arch = "wasm32"))]
use aes_gcm::aead::{rand_core::RngCore, OsRng};
use aes_gcm::{AeadInPlace, Aes256Gcm, Key, KeyInit, Nonce};
use hkdf::{
    hmac::{Hmac, Mac},
    Hkdf,
};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("no encryption key available")]
    NoKeyAvailable,
}

pub const KEY_LEN: usize = 32;
pub const GCM_NONCE_SIZE: usize = 12;

#[derive(Debug, Clone)]
pub(crate) struct Encryption {
    key: Option<[u8; KEY_LEN]>,
}

impl Encryption {
    pub fn new(key: &[u8], info: &[u8]) -> Result<Self, EncryptionError> {
        let key = Encryption::derive_key(key, info)?;
        Ok(Self { key })
    }

    fn derive_key(key: &[u8], info: &[u8]) -> Result<Option<[u8; KEY_LEN]>, EncryptionError> {
        if key.is_empty() {
            return Ok(None);
        }
        let hk = Hkdf::<Sha256>::new(None, key);
        let mut derived = [0u8; KEY_LEN];
        match hk.expand(info, &mut derived) {
            Ok(_) => Ok(Some(derived)),
            Err(e) => Err(EncryptionError::KeyDerivation(format!(
                "HKDF expansion failed: {:?}",
                e
            ))),
        }
    }

    fn make_gcm_cipher(&self, info: &[u8]) -> Result<Aes256Gcm, EncryptionError> {
        let key = match self.key {
            Some(k) => Self::derive_key(&k, info)?,
            None => return Err(EncryptionError::NoKeyAvailable),
        };
        let key = key.ok_or(EncryptionError::KeyDerivation(
            "Failed deriving key".to_string(),
        ))?;
        Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key)))
    }

    /// Generate a secure nonce for encryption
    #[cfg(not(target_arch = "wasm32"))]
    fn generate_nonce() -> [u8; GCM_NONCE_SIZE] {
        let mut nonce = [0u8; GCM_NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    /// Generate a secure nonce in WASM using Web Crypto API
    #[cfg(target_arch = "wasm32")]
    fn generate_nonce() -> [u8; GCM_NONCE_SIZE] {
        use web_sys::window;
        let mut nonce = [0u8; GCM_NONCE_SIZE];

        let crypto = window()
            .expect("No global `window` exists")
            .crypto()
            .expect("No Web Crypto support in this environment");
        crypto
            .get_random_values_with_u8_array(&mut nonce)
            .expect("Failed to get random values");

        nonce
    }

    pub fn encrypt(&self, data: &[u8], info: &[u8]) -> Result<Box<[u8]>, EncryptionError> {
        let gcm = self.make_gcm_cipher(info)?;
        let nonce = Self::generate_nonce();
        let nonce_array = Nonce::from_slice(&nonce);

        let mut buffer = data.to_vec();

        match gcm.encrypt_in_place(nonce_array, b"", &mut buffer) {
            Ok(_) => {
                let mut result = Vec::with_capacity(buffer.len() + GCM_NONCE_SIZE);
                result.extend_from_slice(&buffer);
                result.extend_from_slice(&nonce);

                Ok(result.into_boxed_slice())
            }
            Err(e) => Err(EncryptionError::EncryptionFailed(format!(
                "GCM encryption failed: {:?}",
                e
            ))),
        }
    }

    pub fn encrypt_deterministic(
        &self,
        data: &[u8],
        info: &[u8],
    ) -> Result<Box<[u8]>, EncryptionError> {
        let gcm = self.make_gcm_cipher(info)?;

        let key_bytes = self
            .key
            .as_ref()
            .expect("encrypt_deterministic called without a key");
        // Create HMAC-SHA256(key, data)
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key_bytes)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("HMAC init failed: {:?}", e)))?;
        mac.update(data);
        let hmac_result = mac.finalize().into_bytes();

        // Derive nonce from first gcm.nonce_size() bytes of HMAC
        let nonce_size = GCM_NONCE_SIZE; // usually 12 for AES-GCM
        let nonce_bytes = &hmac_result[..nonce_size];
        let nonce_array = Nonce::from_slice(nonce_bytes);

        // Encrypt in place
        let mut buffer = data.to_vec();
        match gcm.encrypt_in_place(nonce_array, b"", &mut buffer) {
            Ok(_) => {
                // Prepend nonce to ciphertext (like Go's gcm.Seal(nonce, nonce, data, nil))
                let mut result = Vec::with_capacity(nonce_size + buffer.len());
                result.extend_from_slice(nonce_bytes);
                result.extend_from_slice(&buffer);
                Ok(result.into_boxed_slice())
            }
            Err(e) => Err(EncryptionError::EncryptionFailed(format!(
                "GCM encryption failed: {:?}",
                e
            ))),
        }
    }

    /// Decrypts data encrypted with encrypt_deterministic (nonce at start of ciphertext).
    pub fn decrypt_deterministic(
        &self,
        data: &[u8],
        info: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let gcm = self.make_gcm_cipher(info)?;

        if data.len() < GCM_NONCE_SIZE {
            return Err(EncryptionError::DecryptionFailed(
                "Invalid encrypted data: too short".to_string(),
            ));
        }

        let (nonce, ciphertext) = data.split_at(GCM_NONCE_SIZE);
        let nonce_array = Nonce::from_slice(nonce);

        let mut buffer = ciphertext.to_vec();

        match gcm.decrypt_in_place(nonce_array, b"", &mut buffer) {
            Ok(_) => Ok(buffer),
            Err(e) => Err(EncryptionError::DecryptionFailed(format!(
                "GCM decryption failed: {:?}",
                e
            ))),
        }
    }

    pub fn decrypt(&self, data: &[u8], info: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let gcm = self.make_gcm_cipher(info)?;

        if data.len() < GCM_NONCE_SIZE {
            return Err(EncryptionError::DecryptionFailed(
                "Invalid encrypted data: too short".to_string(),
            ));
        }

        let (encrypted_data, nonce) = data.split_at(data.len() - GCM_NONCE_SIZE);
        let nonce_array = Nonce::from_slice(nonce);

        let mut buffer = encrypted_data.to_vec();
        // gcm.decrypt_in_place(nonce_array, b"", &mut buffer)?;

        match gcm.decrypt_in_place(nonce_array, b"", &mut buffer) {
            Ok(_) => Ok(buffer),
            Err(e) => Err(EncryptionError::DecryptionFailed(format!(
                "GCM decryption failed: {:?}",
                e
            ))),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {

    use crate::utils::encryption::Encryption;

    const BUCKET_TO_TEST: &str = "TEST_BUCKET_v2";

    #[tokio::test]
    async fn test_text_encryption() {
        println!("Test 1: Encrypt and decrypt text");

        let data = "This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!!";
        let password = "TestPassword";
        let index: u64 = 1;
        let info = [BUCKET_TO_TEST, "file_name"].join("/");

        let encryption = Encryption::new(password.as_bytes(), info.as_bytes()).unwrap();

        let encrypted = encryption
            .encrypt(data.as_bytes(), &index.to_be_bytes())
            .unwrap();
        let decrypted = encryption
            .decrypt(&encrypted, &index.to_be_bytes())
            .unwrap();
        let decrypted_data = String::from_utf8(decrypted).unwrap();

        assert_eq!(
            decrypted_data, data,
            "checking if original data ({}) and decrypted data ({}) are the same",
            data, decrypted_data
        );
    }

    #[tokio::test]
    async fn test_text_encryption_deterministic() {
        println!("Test 2: Encrypt and Encrypt_deterministic text");

        let data = "This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!!";
        let password = "TestPassword";
        let index: u64 = 1;
        let info = [BUCKET_TO_TEST, "file_name"].join("/");

        let encryption = Encryption::new(password.as_bytes(), info.as_bytes()).unwrap();

        let encrypted_1 = hex::encode(
            encryption
                .encrypt(data.as_bytes(), &index.to_be_bytes())
                .unwrap(),
        );

        let encrypted_2 = hex::encode(
            encryption
                .encrypt(data.as_bytes(), &index.to_be_bytes())
                .unwrap(),
        );

        let encrypted_deterministic_1 = hex::encode(
            encryption
                .encrypt_deterministic(data.as_bytes(), &index.to_be_bytes())
                .unwrap(),
        );

        let encrypted_deterministic_2 = hex::encode(
            encryption
                .encrypt_deterministic(data.as_bytes(), &index.to_be_bytes())
                .unwrap(),
        );

        assert_ne!(
            encrypted_1, encrypted_2,
            "checking if encryption is not deterministic: not deterministic 1 ({}) and not deterministic 2 ({}) are different",
            encrypted_1, encrypted_2,
        );

        assert_eq!(
            encrypted_deterministic_1, encrypted_deterministic_2,
            "checking if encryption_deterministic is deterministic: deterministic 1 ({}) and deterministic 2 ({}) are equal",
            encrypted_deterministic_1, encrypted_deterministic_2,
        );
    }

    #[test]
    fn test_data_overhead() {
        let key = b"test_key_for_data_overhead_check";
        let enc = Encryption::new(key, b"some_info").unwrap();

        for (i, &size) in [1 * 1024 * 1024usize, 4 * 1024 * 1024].iter().enumerate() {
            let data: Vec<u8> = (0..size).map(|j| (j % 251) as u8).collect();
            let encrypted = enc.encrypt(&data, format!("{i}").as_bytes()).unwrap();

            assert_ne!(
                &data[..10],
                &encrypted[..10],
                "encrypted data should differ from plaintext"
            );
            let overhead = encrypted.len() - data.len();
            println!(
                "Data size: {}, Encrypted size: {}, overhead: {}",
                data.len(),
                encrypted.len(),
                overhead
            );
        }
    }
}
