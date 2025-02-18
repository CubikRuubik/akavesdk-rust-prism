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
) -> Result<AesGcm<Aes256, U12>, Box<dyn std::error::Error>> {
    let key = derive_key(origin_key, data)?;
    let new_k: &GenericArray<u8, U32> = Key::<Aes256Gcm>::from_slice(&key);
    let gcm: AesGcm<Aes256, U12> = Aes256Gcm::new(&new_k);
    Ok(gcm)
}

pub fn encrypt(
    key: &[u8],
    data: &[u8],
    info: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut gcm = make_gcm_cipher(key, info)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let mut buffer: Vec<u8> = Vec::new();
    buffer.extend_from_slice(data);

    match gcm.encrypt_in_place(&nonce, b"", &mut buffer) {
        Ok(_) => Ok([nonce.as_slice(), &buffer].concat()),
        Err(_) => Err("Error encrypting data".into()),
    }
}

pub fn decrypt(
    key: &[u8],
    data: &[u8],
    info: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut gcm = make_gcm_cipher(key, info)?;

    let (nonce, encrypted_data) = data.split_at(GCM_NONCE_SIZE);
    let mut buffer: Vec<u8> = Vec::new();
    buffer.extend_from_slice(encrypted_data);

    match gcm.decrypt_in_place(nonce.into(), b"", &mut buffer) {
        Ok(_) => Ok(buffer),
        Err(_) => Err("Error decrypting data".into()),
    }
}
