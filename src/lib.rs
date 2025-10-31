use serde::{Deserialize, Serialize};

pub mod commands;
pub mod crypto;
pub mod git;
pub mod paths;

// Re-export the command functions for integration tests
pub use commands::{decrypt_command, encrypt_command, setup_command};

/// Configuration for a repository's secrets management.
///
/// This struct represents the contents of `.a8c-secrets/config.yaml` and defines
/// which secret files should be managed and where they should be placed when decrypted.
///
/// # Example
///
/// ```yaml
/// sha1: "abc123def456"
/// files:
///   - source: "api-keys/production.env"  # relative to `~/.mobile-secrets`
///     destination: "config/secrets.env"  # relative to the current working directory
///   - source: "certificates/ssl.pem"     # relative to `~/.mobile-secrets`
///     destination: "~/.my-app/ssl-certificate.pem" # relative to the user's home directory
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// SHA1 hash of where the git HEAD of the `~/.mobile-secrets` repository was
    /// when the secrets were last encrypted into `*.enc` files in this repository.
    /// This is automatically updated by the `encrypt` command.
    pub sha1: String,

    /// List of secret files to manage, mapping source files in `~/.mobile-secrets`
    /// to destination paths where they should be placed when decrypted.
    pub files: Vec<SecretFileEntry>,
}

/// Represents a single secret file mapping in the configuration.
///
/// Each entry defines a source file within `~/.mobile-secrets` and a destination
/// path where the decrypted content should be placed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecretFileEntry {
    /// Source file path relative to `~/.mobile-secrets`.
    /// Must be a relative path and cannot contain directory traversal (`../`).
    pub source: String,

    /// Destination path where the decrypted file should be placed.
    /// Can be an absolute or relative path. Relative paths are resolved
    /// relative to the current working directory.
    pub destination: String,
}

// Constants for file names and paths

/// Directory name for repository secrets configuration and encrypted files.
pub const REPO_SECRETS_DIR: &str = ".a8c-secrets";

/// Configuration file name within the repository secrets directory.
pub const REPO_SECRETS_CONFIG_FILE: &str = "config.yaml";

/// Encryption keys file name within `~/.mobile-secrets`.
pub const MOBILE_SECRETS_ENCRYPTION_KEYS_FILE: &str = "a8c-secrets-encryption-keys.yaml";

/// Environment variable name for providing the encryption key in CI/CD environments.
pub const ENV_VAR_KEY: &str = "A8C_SECRETS_ENCRYPTION_KEY";
