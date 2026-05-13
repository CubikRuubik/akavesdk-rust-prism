pub mod cids;
pub mod dag;
pub mod encryption;
pub mod erasure;
pub mod http_ext;
pub mod io;
pub mod pb_data;
pub mod peer_id;
pub mod retry;
pub mod streamenc;
pub mod timestamp;

pub fn calculate_bucket_id(bucket_name: &str, address_hex: &str) -> [u8; 32] {
    use web3::signing::keccak256;
    let addr_bytes = hex::decode(address_hex).unwrap_or_default();
    let addr_20: [u8; 20] = if addr_bytes.len() >= 20 {
        let start = addr_bytes.len() - 20;
        addr_bytes[start..].try_into().unwrap()
    } else {
        let mut padded = [0u8; 20];
        let start = 20 - addr_bytes.len();
        padded[start..].copy_from_slice(&addr_bytes);
        padded
    };
    let mut data = Vec::new();
    data.extend_from_slice(bucket_name.as_bytes());
    data.extend_from_slice(&addr_20);
    keccak256(&data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn generate_nonce() -> [u8; 32] {
    use aes_gcm::aead::{rand_core::RngCore, OsRng};
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{calculate_bucket_id, generate_nonce};

    #[test]
    fn test_calculate_bucket_id() {
        let cases = [
            (
                "test1",
                "eea1eddf9f4be315e978c6d0d25d1b870ec0162ebf0acf173f47b738ff0cb421",
                "7d8b15e57405638fe772de6bb73b94345deb1f41fa1850654bc1f587a5a6afa7",
            ),
            (
                "bucket new",
                "eea1eddf9f4be315e978c6d0d25d1b870ec0162ebf0acf173f47b738ff0cb421",
                "ca7b393db299deee1bf58fcb9670b9e6e6079cba1e85bca7c62dbd889caba925",
            ),
            (
                "random name",
                "eea1eddf9f4be315e978c6d0d25d1b870ec0162ebf0acf173f47b738ff0cb421",
                "8f92db9fde643ed88b4dc2e238e329bafdff4a172b34d0501c2f46a0d2c36696",
            ),
        ];
        for (name, address, expected) in &cases {
            let result = calculate_bucket_id(name, address);
            assert_eq!(hex::encode(result), *expected, "bucket_name={name}");
        }
    }

    #[test]
    fn test_generate_nonce() {
        let mut nonce = [0u8; 32];
        for _ in 0..10 {
            nonce = generate_nonce();
            if nonce[0] != 0 {
                break;
            }
        }
        assert_eq!(nonce.len(), 32);
        assert_ne!(
            nonce[0], 0,
            "nonce leading byte should be non-zero within 10 retries"
        );

        let another = generate_nonce();
        assert_ne!(nonce, another, "consecutive nonces must not repeat");
    }
}
