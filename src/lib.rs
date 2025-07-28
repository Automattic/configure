use serde::{Deserialize, Serialize};

pub mod commands;
pub mod crypto;
pub mod git;
pub mod paths;

// Re-export the command functions for integration tests
pub use commands::{decrypt_command, encrypt_command, setup_command};

// Re-export the Config struct
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub sha1: String,
    pub files: Vec<SecretFileEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SecretFileEntry {
    pub source: String,
    pub destination: String,
}

// Constants for file names
pub const REPO_SECRETS_DIR: &str = ".a8c-secrets";
pub const REPO_SECRETS_CONFIG_FILE: &str = "config.yaml";
pub const MOBILE_SECRETS_ENCRYPTION_KEYS_FILE: &str = "a8c-secrets-encryption-keys.yaml";
pub const ENV_VAR_KEY: &str = "A8C_SECRETS_ENCRYPTION_KEY";
