use aes_gcm::{AeadInPlace, Aes256Gcm, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

#[cfg(not(target_arch = "wasm32"))]
use aes_gcm::aead::{OsRng, rand_core::RngCore};

pub const KEY_LEN: usize = 32;
pub const GCM_NONCE_SIZE: usize = 12;

#[derive(Debug)]
pub struct Encryption {
    key: Option<[u8; KEY_LEN]>,
}

impl Encryption {
    pub fn new(key: &[u8], info: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let key = Encryption::derive_key(key, info)?;
        Ok(Self { key })
    }

    pub fn len(&self) -> usize {
        self.key.map(|k| k.len()).unwrap_or(0)
    }

    fn derive_key(
        key: &[u8],
        info: &[u8],
    ) -> Result<Option<[u8; KEY_LEN]>, Box<dyn std::error::Error>> {
        if key.is_empty() {
            return Ok(None);
        }
        let hk = Hkdf::<Sha256>::new(None, key);
        let mut derived = [0u8; KEY_LEN];
        match hk.expand(info, &mut derived) {
            Ok(_) => Ok(Some(derived)),
            Err(_) => Err(Box::from("hk.expand failed: invalid length")),
        }
    }

    fn make_gcm_cipher(&self, info: &[u8]) -> Result<Aes256Gcm, Box<dyn std::error::Error>> {
        let key = match self.key {
            Some(k) => Self::derive_key(&k, info)?,
            None => return Err("No encryption key available".into()),
        };
        let key = key.ok_or("Failed deriving key")?;
        Ok(Aes256Gcm::new(&Key::<Aes256Gcm>::from_slice(&key)))
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

    pub fn encrypt(
        &self,
        data: &[u8],
        info: &[u8],
    ) -> Result<Box<[u8]>, Box<dyn std::error::Error>> {
        let mut gcm = self.make_gcm_cipher(info)?;
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
            Err(e) => Err(Box::from("gcm encrypt error")),
        }
    }

    pub fn decrypt(&self, data: &[u8], info: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut gcm = self.make_gcm_cipher(info)?;

        if data.len() < GCM_NONCE_SIZE {
            return Err("Invalid encrypted data".into());
        }

        let (encrypted_data, nonce) = data.split_at(data.len() - GCM_NONCE_SIZE);
        let nonce_array = Nonce::from_slice(nonce);

        let mut buffer = encrypted_data.to_vec();
        // gcm.decrypt_in_place(nonce_array, b"", &mut buffer)?;

        match gcm.decrypt_in_place(nonce_array, b"", &mut buffer) {
            Ok(_) => Ok(buffer),
            Err(_) => Err(Box::from("gcm decrypt error")),
        }
    }
}

mod tests {

    use crate::utils::encryption::Encryption;

    const BUCKET_TO_TEST: &str = "TEST_BUCKET_v2";

    async fn test_text_encryption() {
        println!("Test 1: Encrypt and decrypt text");

        let data = "This is a phrase to test!! This is a phrase to test!! This is a phrase to test!! This is a phrase to test!!";
        let password = "TestPassword";
        let index: u64 = 1;
        let info = vec![BUCKET_TO_TEST, "file_name"].join("/");

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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_all() {
        test_text_encryption().await;
    }
}
