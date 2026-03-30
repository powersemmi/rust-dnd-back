use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng, rand_core::RngCore},
};
use base64::{Engine as _, engine::general_purpose};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

const AES_KEY_LEN: usize = 32;
const AES_GCM_NONCE_LEN: usize = 12;
const KDF_SALT_LEN: usize = 16;
const PBKDF2_ITERATIONS: u32 = 100_000;
const ENCRYPTION_FORMAT_VERSION: u8 = 1;

fn derive_key(secret: &str, salt: &[u8]) -> [u8; AES_KEY_LEN] {
    let mut key = [0_u8; AES_KEY_LEN];
    pbkdf2_hmac::<Sha256>(secret.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn encrypt(data: &str, secret: &str) -> Result<String, String> {
    let mut salt = [0_u8; KDF_SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let key = derive_key(secret, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|error| format!("Cipher initialization failure: {error}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|error| format!("Encryption failure: {error}"))?;

    let mut combined = Vec::with_capacity(1 + KDF_SALT_LEN + AES_GCM_NONCE_LEN + ciphertext.len());
    combined.push(ENCRYPTION_FORMAT_VERSION);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(combined))
}

pub fn decrypt(encrypted_data: &str, secret: &str) -> Result<String, String> {
    let encrypted_bytes = general_purpose::STANDARD
        .decode(encrypted_data)
        .map_err(|error| format!("Base64 decode error: {error}"))?;

    if encrypted_bytes.first() != Some(&ENCRYPTION_FORMAT_VERSION) {
        return Err("Unsupported encryption format version".to_string());
    }

    decrypt_v1(&encrypted_bytes, secret)
}

fn decrypt_v1(encrypted_bytes: &[u8], secret: &str) -> Result<String, String> {
    let minimum_len = 1 + KDF_SALT_LEN + AES_GCM_NONCE_LEN;
    if encrypted_bytes.len() <= minimum_len {
        return Err("Data too short".to_string());
    }

    let salt_start = 1;
    let nonce_start = salt_start + KDF_SALT_LEN;
    let ciphertext_start = nonce_start + AES_GCM_NONCE_LEN;

    let salt = &encrypted_bytes[salt_start..nonce_start];
    let nonce = Nonce::from_slice(&encrypted_bytes[nonce_start..ciphertext_start]);
    let ciphertext = &encrypted_bytes[ciphertext_start..];

    let key = derive_key(secret, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|error| format!("Cipher initialization failure: {error}"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|error| format!("Decryption failure: {error}"))?;

    String::from_utf8(plaintext).map_err(|error| format!("UTF-8 error: {error}"))
}
#[cfg(test)]
mod tests {
    use super::{AES_GCM_NONCE_LEN, ENCRYPTION_FORMAT_VERSION, decrypt, encrypt, general_purpose};
    use base64::Engine as _;

    const SECRET: &str = "test-auth-secret-1234567890";
    const PLAINTEXT: &str = "totp-secret-value";

    #[test]
    fn encrypt_decrypt_round_trip() {
        let encrypted = encrypt(PLAINTEXT, SECRET).expect("encryption must succeed");
        let decrypted = decrypt(&encrypted, SECRET).expect("decryption must succeed");

        assert_eq!(decrypted, PLAINTEXT);
    }

    #[test]
    fn decrypt_rejects_unsupported_format_version() {
        let payload = general_purpose::STANDARD.encode([0_u8; AES_GCM_NONCE_LEN + 1]);
        let error = decrypt(&payload, SECRET).expect_err("payload must be rejected");

        assert!(error.contains("Unsupported encryption format version"));
    }

    #[test]
    fn decrypt_rejects_too_short_payload() {
        let mut raw = vec![ENCRYPTION_FORMAT_VERSION];
        raw.extend_from_slice(&[0_u8; AES_GCM_NONCE_LEN - 1]);
        let payload = general_purpose::STANDARD.encode(raw);
        let error = decrypt(&payload, SECRET).expect_err("payload must be rejected");

        assert!(error.contains("too short"));
    }
}
