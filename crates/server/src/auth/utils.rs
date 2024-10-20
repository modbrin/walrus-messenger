use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn hash_password_sha256(password: &str, salt: [u8; 16]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(password.as_bytes());
    hash.update(&salt);
    hash.finalize().into()
}

pub fn generate_salt() -> [u8; 16] {
    // uuid is conveniently 128 bits
    Uuid::new_v4().as_u128().to_le_bytes()
}
