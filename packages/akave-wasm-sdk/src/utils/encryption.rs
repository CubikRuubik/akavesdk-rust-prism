use aes_gcm::{
    aead::{AeadMutInPlace, OsRng},
    aes::Aes256,
    AeadCore, Aes256Gcm, AesGcm, Key, KeyInit,
};
use hkdf::{Hkdf, InvalidLength};
use sha2::{
    digest::{
        generic_array::GenericArray,
        typenum::consts::{U12, U32},
    },
    Sha256,
};

pub const KEY_LEN: usize = 32;
pub const GCM_NONCE_SIZE: usize = 12;

pub struct Encryption {
    key: Option<[u8; KEY_LEN]>,
}

impl Encryption {
    pub fn new(key: &[u8], info: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let key = Encryption::derive_key(key, info)?;
        Ok(Self { key })
    }

    pub fn len(&self) -> usize {
        match self.key {
            Some(size) => size.len(),
            None => 0,
        }
    }

    // call in the sdk with
    // let info = vec![bucket_name, file_name].join("/");
    // and key as private key?
    fn derive_key(
        key: &[u8],
        info: &[u8],
    ) -> Result<Option<[u8; KEY_LEN]>, Box<dyn std::error::Error>> {
        // let password_byte = key.as_bytes();
        if key.len() == 0 {
            return Ok(None);
        }

        let hk: Hkdf<Sha256> = Hkdf::<Sha256>::new(None, key);
        let mut derived = [0u8; KEY_LEN];
        let res: Result<(), InvalidLength> = hk.expand(info, &mut derived);
        match res {
            Ok(_) => Ok(Some(derived)),
            Err(_) => Err(format!("{} is a valid length for Sha256 to output", KEY_LEN).into()),
        }
    }

    fn make_gcm_cipher(
        &self,
        data: &[u8],
    ) -> Result<AesGcm<Aes256, U12>, Box<dyn std::error::Error>> {
        match self.key {
            Some(some_key) => {
                let key = Encryption::derive_key(&some_key, data)?;
                let new_k: &GenericArray<u8, U32> = Key::<Aes256Gcm>::from_slice(&some_key);
                let gcm: AesGcm<Aes256, U12> = Aes256Gcm::new(&new_k);
                Ok(gcm)
            }
            None => Err("There's no saved key")?,
        }
    }

    pub fn encrypt(&self, data: &[u8], info: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut gcm = self.make_gcm_cipher(info)?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut buffer: Vec<u8> = Vec::new();
        buffer.extend_from_slice(data);

        match gcm.encrypt_in_place(&nonce, b"", &mut buffer) {
            Ok(_) => Ok([&buffer, nonce.as_slice()].concat()),
            Err(_) => Err("Error encrypting data".into()),
        }
    }

    pub fn decrypt(&self, data: &[u8], info: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut gcm = self.make_gcm_cipher(info)?;

        if let Some((encrypted_data, nonce)) = data.split_last_chunk::<GCM_NONCE_SIZE>() {
            let mut buffer: Vec<u8> = Vec::new();
            buffer.extend_from_slice(encrypted_data);

            return match gcm.decrypt_in_place(nonce.into(), b"", &mut buffer) {
                Ok(_) => Ok(buffer),
                Err(_) => Err("Error decrypting data".into()),
            };
        }
        Err("Error decrypting data".into())
    }
}

mod tests {

    use crate::utils::encryption::{self, Encryption};

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

    #[tokio::test]
    async fn test_all() {
        test_text_encryption().await;
    }
}
