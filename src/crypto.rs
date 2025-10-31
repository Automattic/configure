use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{ENV_VAR_KEY, MOBILE_SECRETS_ENCRYPTION_KEYS_FILE};

use tracing::debug;

/// AES-256-GCM nonce size in bytes
pub const AES_GCM_NONCE_SIZE: usize = 12;

/// AES-256 key size in bytes
pub const AES_256_KEY_SIZE: usize = 32;

/// Minimum size for encrypted data (nonce + at least 1 byte of ciphertext)
pub const MIN_ENCRYPTED_DATA_SIZE: usize = AES_GCM_NONCE_SIZE + 1;

/// Retrieves the encryption key for the specified repository.
///
/// This function looks for the encryption key in the following order:
/// 1. Environment variable `A8C_SECRETS_ENCRYPTION_KEY` (for CI/CD environments)
/// 2. `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml` file
///
/// # Arguments
/// - `repo_name` - Name of the repository to get the encryption key for
/// - `mobile_secrets_path` - Path to the mobile-secrets directory
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the decrypted encryption key
/// - `Err(anyhow::Error)` if key retrieval or decryption fails
pub fn get_encryption_key_for_repo(repo_name: &str, mobile_secrets_path: &Path) -> Result<Vec<u8>> {
    // First, check if the key is provided via environment variable (for CI/CD)
    if let Ok(key_b64) = std::env::var(ENV_VAR_KEY) {
        debug!("Using encryption key from environment variable");
        return general_purpose::STANDARD.decode(key_b64).map_err(|e| {
            anyhow!(
                "Invalid base64 encoding in {} environment variable: {}\n\n\
                The encryption key must be a valid base64-encoded string.",
                ENV_VAR_KEY,
                e
            )
        });
    }

    // Otherwise, load from the keys file
    let keys_file_path = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);

    if !keys_file_path.exists() {
        return Err(anyhow!(
            "Encryption keys file not found: {}\n\n\
            This repository has not been set up for a8c-secrets yet.\n\
            Run 'a8c-secrets setup' to initialize the configuration and generate encryption keys.",
            keys_file_path.display()
        ));
    }

    let keys_content = fs::read_to_string(&keys_file_path).map_err(|e| {
        anyhow!(
            "Failed to read encryption keys file {}: {}\n\n\
            Please check that you have read permissions for the file.",
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

    let key_b64 = keys.get(repo_name).ok_or_else(|| {
        anyhow!(
            "No encryption key found for repository '{}' in {}\n\n\
            This repository has not been set up for a8c-secrets yet.\n\
            Run 'a8c-secrets setup' to initialize the configuration and generate encryption keys.",
            repo_name,
            keys_file_path.display()
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
#[must_use]
pub fn generate_encryption_key() -> [u8; AES_256_KEY_SIZE] {
    use rand::RngCore;
    let mut key = [0u8; AES_256_KEY_SIZE];
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

    let mut nonce_bytes = [0u8; AES_GCM_NONCE_SIZE];
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

    if encrypted_data.len() < MIN_ENCRYPTED_DATA_SIZE {
        return Err(anyhow!(
            "Invalid encrypted data: too short (expected at least {} bytes, got {})",
            MIN_ENCRYPTED_DATA_SIZE,
            encrypted_data.len()
        ));
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&encrypted_data[0..AES_GCM_NONCE_SIZE]);
    let ciphertext = &encrypted_data[AES_GCM_NONCE_SIZE..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use tempfile;

    #[test]
    fn test_generate_encryption_key() {
        let key1 = generate_encryption_key();
        let key2 = generate_encryption_key();

        // Keys should be different (random)
        assert_ne!(key1, key2);

        // Keys should be 32 bytes (AES-256)
        assert_eq!(key1.len(), AES_256_KEY_SIZE);
        assert_eq!(key2.len(), AES_256_KEY_SIZE);
    }

    #[test]
    fn test_encrypt_decrypt_data() {
        let key = generate_encryption_key();
        let plaintext = b"Hello, World! This is a test message.";

        // Encrypt
        let encrypted = encrypt_data(plaintext, &key).unwrap();

        // Verify encrypted data is different from plaintext
        assert_ne!(encrypted, plaintext);

        // Verify encrypted data is longer than plaintext (due to nonce + tag)
        assert!(encrypted.len() > plaintext.len());

        // Decrypt
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        // Verify decrypted data matches original
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let key = generate_encryption_key();
        let plaintext = b"";

        let encrypted = encrypt_data(plaintext, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key = generate_encryption_key();
        let plaintext: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let encrypted = encrypt_data(&plaintext, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let key1 = generate_encryption_key();
        let key2 = generate_encryption_key();
        let plaintext = b"Test message";

        let encrypted = encrypt_data(plaintext, &key1).unwrap();

        // Should fail with wrong key
        let result = decrypt_data(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_data() {
        let key = generate_encryption_key();
        let plaintext = b"Test message";

        let mut encrypted = encrypt_data(plaintext, &key).unwrap();

        // Corrupt the data
        if !encrypted.is_empty() {
            encrypted[0] ^= 1;
        }

        let result = decrypt_data(&encrypted, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short_data() {
        let key = generate_encryption_key();
        let short_data = vec![0u8; MIN_ENCRYPTED_DATA_SIZE - 1];

        let result = decrypt_data(&short_data, &key);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let key = generate_encryption_key();
        let invalid_base64 = b"not-valid-base64!@#";
        let result = decrypt_data(invalid_base64, &key);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // The function doesn't validate base64 first, so it fails with decryption error
        assert!(error_msg.contains("Decryption failed") || error_msg.contains("aead::Error"));
    }

    #[test]
    fn test_get_encryption_key_for_repo_with_env_var() {
        // Generate a random 32-byte key and base64-encode it
        let mut raw_key = [0u8; AES_256_KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut raw_key);
        let test_key = base64::engine::general_purpose::STANDARD.encode(raw_key);

        // Store original value to restore later
        let original_value = std::env::var(ENV_VAR_KEY).ok();

        // Set the environment variable
        std::env::set_var(ENV_VAR_KEY, &test_key);

        // Create a temporary directory for mobile-secrets
        let temp_dir = tempfile::tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path().join(".mobile-secrets");
        std::fs::create_dir_all(&mobile_secrets_path).unwrap();

        let result = get_encryption_key_for_repo("test-repo", &mobile_secrets_path);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), AES_256_KEY_SIZE);
        assert_eq!(key, raw_key);

        // Restore original environment state
        match original_value {
            Some(val) => std::env::set_var(ENV_VAR_KEY, val),
            None => std::env::remove_var(ENV_VAR_KEY),
        }
    }

    #[test]
    fn test_get_encryption_key_for_repo_without_env_var() {
        // Store original value to restore later
        let original_value = std::env::var(ENV_VAR_KEY).ok();

        // Ensure environment variable is not set
        std::env::remove_var(ENV_VAR_KEY);

        // Create a temporary directory for mobile-secrets (empty, so no keys file)
        let temp_dir = tempfile::tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path().join(".mobile-secrets");
        std::fs::create_dir_all(&mobile_secrets_path).unwrap();

        // This should fail because there's no keys file in the temp directory
        let result = get_encryption_key_for_repo("test-repo", &mobile_secrets_path);
        assert!(result.is_err());

        // Restore original environment state
        match original_value {
            Some(val) => std::env::set_var(ENV_VAR_KEY, val),
            None => std::env::remove_var(ENV_VAR_KEY),
        }
    }

    #[test]
    fn test_get_encryption_key_for_repo_from_keys_file() {
        // Store original value to restore later
        let original_value = std::env::var(ENV_VAR_KEY).ok();

        // Ensure environment variable is not set
        std::env::remove_var(ENV_VAR_KEY);

        // Create a temporary directory for mobile-secrets
        let temp_dir = tempfile::tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path().join(".mobile-secrets");
        std::fs::create_dir_all(&mobile_secrets_path).unwrap();

        // Create a keys file with a test key
        let keys_file_path = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
        let test_key_b64 = "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="; // "test-key-for-testing-purposes-32"
        let keys_content = format!("test-repo: {test_key_b64}");
        std::fs::write(&keys_file_path, keys_content).unwrap();

        // This should succeed and return the key from the file
        let result = get_encryption_key_for_repo("test-repo", &mobile_secrets_path);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), AES_256_KEY_SIZE);

        // Verify the key matches what we put in the file
        let expected_key = base64::engine::general_purpose::STANDARD
            .decode(test_key_b64)
            .unwrap();
        assert_eq!(key, expected_key);

        // Test that requesting a non-existent repo fails
        let result = get_encryption_key_for_repo("non-existent-repo", &mobile_secrets_path);
        assert!(result.is_err());

        // Restore original environment state
        match original_value {
            Some(val) => std::env::set_var(ENV_VAR_KEY, val),
            None => std::env::remove_var(ENV_VAR_KEY),
        }
    }

    #[test]
    fn test_constants() {
        // Verify our constants are correct
        assert_eq!(AES_GCM_NONCE_SIZE, 12);
        assert_eq!(AES_256_KEY_SIZE, 32);
        assert_eq!(MIN_ENCRYPTED_DATA_SIZE, AES_GCM_NONCE_SIZE + 1); // nonce + minimum data size
    }
}
