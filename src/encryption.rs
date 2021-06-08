use crate::ConfigureError;
use log::debug;
use sodiumoxide::base64::Variant;
use sodiumoxide::base64::{decode, encode};
use sodiumoxide::crypto::secretbox;
use std::fs::{read, write};
use std::path::Path;

pub fn init() {
    sodiumoxide::init().expect("Unable to initialize libsodium");
}

pub fn generate_key() -> String {
    debug!("Generating an encryption key");
    let key_bytes = secretbox::gen_key();
    encode_key(key_bytes)
}

pub fn encrypt_file(
    input_path: &Path,
    output_path: &Path,
    key: &EncryptionKey
) -> Result<(), ConfigureError> {
    let file_contents = match read(input_path) {
        Ok(file_contents) => file_contents,
        Err(_err) => return Err(ConfigureError::InputFileNotReadable)
    };

    let encrypted_bytes = match encrypt_bytes(file_contents, key) {
        Ok(encrypted_bytes) => encrypted_bytes,
        Err(_err)           => return Err(ConfigureError::DataEncryptionError)
    };

    match write(&output_path, encrypted_bytes) {
        Ok(()) => Ok(()),
        Err(_err) => Err(ConfigureError::OutputFileNotWritable)
    }
}

pub fn decrypt_file(
    input_path: &Path,
    output_path: &Path,
    key: &EncryptionKey
) -> Result<(), ConfigureError> {
    let file_contents = match read(input_path) {
        Ok(file_contents) => file_contents,
        Err(_err) => return Err(ConfigureError::InputFileNotReadable)
    };

    let decrypted_bytes = match decrypt_bytes(file_contents, key) {
        Ok(decrypted_bytes) => decrypted_bytes,
        Err(_err) => return Err(ConfigureError::DataDecryptionError),
    };

    match write(&output_path, decrypted_bytes) {
        Ok(()) => Ok(()),
        Err(_err) => Err(ConfigureError::OutputFileNotWritable)
    }
}

fn encrypt_bytes(input: Vec<u8>, key: &EncryptionKey) -> Result<Vec<u8>, ConfigureError> {
    let nonce = secretbox::gen_nonce();
    let secret_bytes = secretbox::seal(&input, &nonce, &key.key);

    Ok([&nonce[..], &secret_bytes].concat())
}

fn decrypt_bytes(input: Vec<u8>, key: &EncryptionKey) -> Result<Vec<u8>, ConfigureError> {
    // Encoded Format byte layout:
    // |======================================|=====================================|
    // | 0                                 23 | 24                                ∞ |
    // |======================================|=====================================|
    // |                nonce                 |           encrypted data            |
    // |======================================|=====================================|

    const NONCE_SIZE: usize = 24;

    // Read the nonce bytes
    let mut nonce_bytes: [u8; NONCE_SIZE] = Default::default();
    nonce_bytes.copy_from_slice(&input[0..NONCE_SIZE]);
    let nonce = sodiumoxide::crypto::secretbox::Nonce(nonce_bytes);

    // Read the encrypted data bytes
    let data_bytes = &input[NONCE_SIZE..];

    let decrypted_bytes = match secretbox::open(data_bytes, &nonce, &key.key) {
        Ok(decrypted_bytes) => decrypted_bytes,
        Err(_)              => return Err(ConfigureError::DataDecryptionError),
    };

    Ok(decrypted_bytes)
}

fn encode_key(key: sodiumoxide::crypto::secretbox::Key) -> String {
    encode(&key, Variant::Original)
}

fn decode_key(key: &str) -> Result<EncryptionKey, ConfigureError> {
    match decode(key.trim(), Variant::Original) {
        Ok(decoded_key) => {
            if decoded_key.len() != 32 {
                return Err(ConfigureError::DecryptionKeyParsingError);
            }

            let mut key_bytes: [u8; 32] = Default::default();
            key_bytes.copy_from_slice(&decoded_key);
            let raw_key = sodiumoxide::crypto::secretbox::Key(key_bytes);

            Ok(EncryptionKey {
                key: raw_key
            })
        }
        Err(_err) => Err(ConfigureError::DecryptionKeyEncodingError),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct EncryptionKey {
    pub key: sodiumoxide::crypto::secretbox::Key
}

impl From<sodiumoxide::crypto::secretbox::Key> for EncryptionKey {
    fn from(raw_key: sodiumoxide::crypto::secretbox::Key) -> EncryptionKey {
        EncryptionKey {
            key: raw_key
        }
    }
}

impl EncryptionKey {

    pub fn from_str(encryption_key: &str) -> Result<EncryptionKey, ConfigureError> {
        match decode_key(encryption_key) {
            Ok(encryption_key) => Ok(encryption_key as EncryptionKey),
            Err(err) => Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    // Import the parent scope
    use super::*;

    #[test]
    fn test_that_decode_key_succeeds_for_valid_key() {
        assert!(decode_key("B6EeQVtVMBvtZQxEFruq8bUrlPqjtfYdxv2NpL18w1o=").is_ok())
    }

    #[test]
    fn test_that_decode_key_does_not_fail_for_trailing_whitespace() {
        assert!(decode_key("B6EeQVtVMBvtZQxEFruq8bUrlPqjtfYdxv2NpL18w1o= ").is_ok())
    }

    #[test]
    fn test_that_decode_key_does_not_fail_for_leading_whitespace() {
        assert!(decode_key(" B6EeQVtVMBvtZQxEFruq8bUrlPqjtfYdxv2NpL18w1o=").is_ok())
    }

    #[test]
    fn test_that_decode_key_fails_for_invalid_base64() {
        assert!(decode_key("Invalid base64").is_err())
    }

    #[test]
    fn test_that_decode_key_returns_exit_code_20() {
        assert_eq!(decode_key("Invalid base64").unwrap_err() as i32, 19);
    }

    #[test]
    fn test_that_decode_key_fails_for_invalid_sodium_key() {
        assert!(decode_key("dGhpcyBpcyBhIHRlc3Q=").is_err())
    }

    #[test]
    fn test_that_decode_key_returns_exit_code_19() {
        assert_eq!(decode_key("dGhpcyBpcyBhIHRlc3Q=").unwrap_err() as i32, 20)
    }
}
