use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;

use crate::{
    ENV_VAR_KEY, MOBILE_SECRETS_ENCRYPTION_KEYS_FILE,
};

use crate::paths::get_mobile_secrets_path;
use crate::git::get_current_repo_name;

/// Retrieves the encryption key for the current repository.
///
/// Attempts to get the key from:
/// 1. `A8C_SECRETS_ENCRYPTION_KEY` environment variable (base64 encoded)
/// 2. `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml` file
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the raw encryption key bytes
/// - `Err(anyhow::Error)` if neither source is available, key is not found for this repo, or base64 decoding fails
pub fn get_encryption_key_for_current_repo() -> Result<Vec<u8>> {
    // Try to get encryption key from environment variable first
    if let Ok(key_b64) = std::env::var(ENV_VAR_KEY) {
        return general_purpose::STANDARD.decode(key_b64).map_err(|e| {
            anyhow!(
                "Invalid base64 encoding in {} environment variable: {}\n\n\
                The encryption key must be a valid base64-encoded string.",
                ENV_VAR_KEY,
                e
            )
        });
    }

    // Otherwise, load from keys file
    let mobile_secrets_path = get_mobile_secrets_path()?;
    let keys_file_path = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);

    if !keys_file_path.exists() {
        return Err(anyhow!(
            "No encryption key available for this repository.\n\n\
            Neither {} environment variable is set nor {} exists.\n\n\
            For local development: Run 'a8c-secrets setup' to generate an encryption key.\n\
            For CI: Set the {} environment variable with the repository's encryption key.",
            ENV_VAR_KEY,
            keys_file_path.display(),
            ENV_VAR_KEY
        ));
    }

    let keys_content = fs::read_to_string(&keys_file_path).map_err(|e| {
        anyhow!(
            "Failed to read encryption keys file {}: {}\n\n\
            Please check that the file exists and you have read permissions.",
            keys_file_path.display(),
            e
        )
    })?;

    let keys: HashMap<String, String> = serde_yaml::from_str(&keys_content).map_err(|e| {
        anyhow!(
            "Invalid YAML syntax in encryption keys file {}: {}\n\n\
            The file should contain a mapping of repository names to base64-encoded keys.",
            keys_file_path.display(),
            e
        )
    })?;

    let repo_name = get_current_repo_name()?;
    let key_b64 = keys.get(&repo_name).ok_or_else(|| {
        anyhow!(
            "No encryption key found for repository '{}' in {}.\n\n\
            Available repositories: {}\n\n\
            Run 'a8c-secrets setup' to generate an encryption key for this repository.",
            repo_name,
            keys_file_path.display(),
            if keys.is_empty() {
                "none".to_string()
            } else {
                keys.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        )
    })?;

    general_purpose::STANDARD.decode(key_b64).map_err(|e| {
        anyhow!(
            "Invalid base64 encoding for repository '{}' in {}: {}\n\n\
            The encryption key must be a valid base64-encoded string.",
            repo_name,
            keys_file_path.display(),
            e
        )
    })
}

/// Generates a cryptographically secure random 256-bit (32-byte) encryption key.
///
/// # Returns
/// A 32-byte array containing random bytes suitable for AES-256 encryption
pub fn generate_encryption_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Encrypts data using AES-256-GCM encryption.
///
/// The output format is: [12-byte nonce][encrypted data]
///
/// # Arguments
/// - `data` - The plaintext data to encrypt
/// - `key` - The 32-byte AES-256 encryption key
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the nonce concatenated with the encrypted data
/// - `Err(anyhow::Error)` if encryption fails
pub fn encrypt_data(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
    use rand::RngCore;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypts data that was encrypted with `encrypt_data`.
///
/// Expects the input format: [12-byte nonce][encrypted data]
///
/// # Arguments
/// - `encrypted_data` - The encrypted data with nonce prefix
/// - `key` - The 32-byte AES-256 decryption key
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the decrypted plaintext data
/// - `Err(anyhow::Error)` if the data is too short, decryption fails, or authentication fails
pub fn decrypt_data(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};

    if encrypted_data.len() < 12 {
        return Err(anyhow!("Invalid encrypted data: too short"));
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&encrypted_data[0..12]);
    let ciphertext = &encrypted_data[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    Ok(plaintext)
}
