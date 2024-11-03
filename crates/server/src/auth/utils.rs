use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::models::session::SessionId;

pub fn hash_password_sha256(password: &str, salt: [u8; 16]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(password.as_bytes());
    hash.update(&salt);
    hash.finalize().into()
}

#[inline]
fn secure_random_bytes<const S: usize>() -> [u8; S] {
    let mut buf = [0u8; S];
    OsRng.fill_bytes(&mut buf);
    buf
}

#[inline]
pub fn generate_salt() -> [u8; 16] {
    secure_random_bytes()
}

#[inline]
pub fn generate_session_token() -> [u8; 32] {
    secure_random_bytes()
}

pub const REFRESH_TOKEN_TTL: chrono::Duration = chrono::Duration::days(14);
pub const ACCESS_TOKEN_TTL: chrono::Duration = chrono::Duration::hours(2);

#[inline]
pub fn new_refresh_token_expiration() -> DateTime<Utc> {
    (current_time().naive_utc() + REFRESH_TOKEN_TTL).and_utc()
}

#[inline]
pub fn new_access_token_expiration() -> DateTime<Utc> {
    (current_time().naive_utc() + ACCESS_TOKEN_TTL).and_utc()
}

#[inline]
pub fn current_time() -> DateTime<Utc> {
    Utc::now()
}

pub fn pack_session_id_and_token(session_id: &SessionId, token: &[u8]) -> Vec<u8> {
    let sid_len = size_of::<SessionId>();
    let mut out = Vec::new();
    out.resize(sid_len + token.len(), 0);
    out[..sid_len].copy_from_slice(session_id.as_bytes());
    out[sid_len..].copy_from_slice(token);
    out
}

pub fn unpack_session_id_and_token(packed: &[u8]) -> Option<(SessionId, &[u8])> {
    let sid_len = size_of::<SessionId>();
    let session_id = SessionId::from_slice(packed.get(..sid_len)?).ok()?;
    let token = packed.get(sid_len..)?;
    Some((session_id, token))
}
