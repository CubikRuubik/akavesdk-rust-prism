use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, Key, KeyInit,
};
use hkdf::{Hkdf, InvalidLength};
use sha2::Sha256;

pub const KEY_LEN: usize = 32;
pub const gcm_nonce_size: usize = 12;

// call in the sdk with
// let info = vec![bucket_name, file_name].join("/");
// and key as private key?
pub fn derive_key(key: &[u8], info: &[u8]) -> Result<[u8; KEY_LEN], Box<dyn std::error::Error>> {
    // let password_byte = key.as_bytes();

    let hk: Hkdf<Sha256> = Hkdf::<Sha256>::new(None, key);
    let mut derived = [0u8; KEY_LEN];
    let res: Result<(), InvalidLength> = hk.expand(info, &mut derived);
    match res {
        Ok(_) => Ok(derived),
        Err(_) => Err(format!("{} is a valid length for Sha256 to output", KEY_LEN).into()),
    }
}

fn make_gcm_cipher(
    origin_key: &[u8],
    data: &[u8],
) -> Result<Aes256Gcm, Box<dyn std::error::Error>> {
    let key = derive_key(origin_key, data)?;
    Ok(Aes256Gcm::new(&key.try_into()?))
}

pub fn encrypt(key: &[u8], data: &[u8], index: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let gcm = make_gcm_cipher(key, &index.to_be_bytes())?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    match gcm.encrypt(&nonce, data) {
        Ok(encrypt_data) => Ok(encrypt_data),
        Err(_) => Err("Error encrypting data".into()),
    }
}

pub fn decrypt(
    key: &[u8],
    data: &[u8],
    info: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let gcm = make_gcm_cipher(key, info)?;

    let (nonce, encrypted_data) = data.split_at(gcm_nonce_size);

    match gcm.decrypt(nonce.into(), encrypted_data) {
        Ok(decrypted_data) => Ok(decrypted_data),
        Err(_) => Err("Error decrypting data".into()),
    }
}
