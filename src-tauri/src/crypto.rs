use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;

pub type Key = Zeroizing<[u8; KEY_LEN]>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KdfParams {
    pub algorithm: String,
    pub salt: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl KdfParams {
    pub fn new_random() -> Self {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let (m_cost, t_cost, p_cost) = if cfg!(test) { (8 * 1024, 1, 1) } else { (64 * 1024, 3, 1) };
        Self { algorithm: "argon2id".into(), salt: B64.encode(salt), m_cost, t_cost, p_cost }
    }

    pub fn derive(&self, password: &str) -> AppResult<Key> {
        if self.algorithm != "argon2id" {
            return Err(AppError::Invalid(format!("不支持的密钥派生算法：{}", self.algorithm)));
        }
        let salt = B64.decode(&self.salt).map_err(|_| AppError::Crypto)?;
        let params = Params::new(self.m_cost, self.t_cost, self.p_cost, Some(KEY_LEN))
            .map_err(|_| AppError::Crypto)?;
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        argon
            .hash_password_into(password.as_bytes(), &salt, key.as_mut())
            .map_err(|_| AppError::Crypto)?;
        Ok(key)
    }
}

#[cfg(test)]
pub fn random_key() -> Key {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    OsRng.fill_bytes(key.as_mut());
    key
}

/// Output layout: nonce(24) || ciphertext+tag
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), Payload { msg: plaintext, aad })
        .map_err(|_| AppError::Crypto)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt(key: &[u8; KEY_LEN], data: &[u8], aad: &[u8]) -> AppResult<Zeroizing<Vec<u8>>> {
    if data.len() < NONCE_LEN {
        return Err(AppError::Crypto);
    }
    let (nonce, ct) = data.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad })
        .map(Zeroizing::new)
        .map_err(|_| AppError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper() {
        let key = random_key();
        let ct = encrypt(&key, "密码 Demo.456!@#".as_bytes(), b"id1").unwrap();
        let pt = decrypt(&key, &ct, b"id1").unwrap();
        assert_eq!(pt.as_slice(), "密码 Demo.456!@#".as_bytes());
        assert!(decrypt(&key, &ct, b"id2").is_err());
        let other = random_key();
        assert!(decrypt(&other, &ct, b"id1").is_err());
    }

    #[test]
    fn kdf_is_deterministic() {
        let p = KdfParams::new_random();
        assert_eq!(*p.derive("主密码").unwrap(), *p.derive("主密码").unwrap());
        assert_ne!(*p.derive("主密码").unwrap(), *p.derive("主密码x").unwrap());
    }
}
