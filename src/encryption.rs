use log::debug;
use sodiumoxide::base64::Variant;
use sodiumoxide::base64::{decode, encode};
use sodiumoxide::crypto::secretbox;
use std::fs::{read, write};
use std::io::{Error, ErrorKind};
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
    secret: &str,
) -> Result<(), std::io::Error> {
    let content = read(input_path)?;
    let ciphertext = encrypt_bytes(content, decode_key(secret));
    write(&output_path, &ciphertext)?;

    Ok(())
}

pub fn decrypt_file(
    input_path: &Path,
    output_path: &Path,
    secret: &str,
) -> Result<(), std::io::Error> {
    let content = read(input_path)?;

    match decrypt_bytes(content, decode_key(secret)) {
        Ok(decrypted_bytes) => Ok(write(&output_path, decrypted_bytes)?),
        Err(_err) => Err(Error::new(ErrorKind::InvalidData, "Unable to decrypt file")),
    }
}

fn encrypt_bytes(input: Vec<u8>, key: sodiumoxide::crypto::secretbox::Key) -> Vec<u8> {
    let nonce = secretbox::gen_nonce();
    let secret_bytes = secretbox::seal(&input, &nonce, &key);
    [&nonce[..], &secret_bytes].concat()
}

fn decrypt_bytes(input: Vec<u8>, key: sodiumoxide::crypto::secretbox::Key) -> Result<Vec<u8>, ()> {
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

    secretbox::open(&data_bytes, &nonce, &key)
}

fn encode_key(key: sodiumoxide::crypto::secretbox::Key) -> String {
    encode(&key, Variant::Original)
}

fn decode_key(key: &str) -> sodiumoxide::crypto::secretbox::Key {
    let decoded_key_bytes = decode(key, Variant::Original).expect("Unable to decode key");

    let mut key_bytes: [u8; 32] = Default::default();
    key_bytes.copy_from_slice(&decoded_key_bytes);

    sodiumoxide::crypto::secretbox::Key(key_bytes)
}
