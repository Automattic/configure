use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    Config, ENV_VAR_KEY, MOBILE_SECRETS_ENCRYPTION_KEYS_FILE, REPO_SECRETS_CONFIG_FILE,
    REPO_SECRETS_DIR,
};

/// Gets the path to the ~/.mobile-secrets directory.
///
/// # Returns
/// - `Ok(PathBuf)` containing the path to ~/.mobile-secrets
/// - `Err(anyhow::Error)` if the HOME environment variable is not set
pub fn get_mobile_secrets_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME environment variable not set"))?;
    Ok(PathBuf::from(home).join(".mobile-secrets"))
}

/// Loads the `.a8c-secrets/config.yaml` configuration file from the current directory.
///
/// # Returns
/// - `Ok(Config)` containing the parsed configuration
/// - `Err(anyhow::Error)` if the file doesn't exist, can't be read, or contains invalid YAML
pub fn load_config() -> Result<Config> {
    let config_path = Path::new(REPO_SECRETS_DIR).join(REPO_SECRETS_CONFIG_FILE);

    if !config_path.exists() {
        return Err(anyhow!(
            "Configuration file not found: {}\n\n\
            This repository has not been set up for a8c-secrets yet.\n\
            Run 'a8c-secrets setup' to initialize the configuration.",
            config_path.display()
        ));
    }

    let config_content = fs::read_to_string(&config_path).map_err(|e| {
        anyhow!(
            "Failed to read configuration file {}: {}\n\n\
            Please check that the file exists and you have read permissions.",
            config_path.display(),
            e
        )
    })?;

    let config: Config = serde_yaml::from_str(&config_content).map_err(|e| {
        anyhow!(
            "Invalid YAML syntax in configuration file {}: {}\n\n\
            Please check the file format. Expected structure:\n\
            sha1: <git-sha1>\n\
            files:\n\
            - source: path/to/source.file\n\
              destination: path/to/destination.file",
            config_path.display(),
            e
        )
    })?;

    Ok(config)
}

/// Gets the SHA1 hash of the current HEAD commit in the ~/.mobile-secrets repository.
///
/// # Arguments
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(String)` containing the SHA1 hash of the HEAD commit
/// - `Err(anyhow::Error)` if the repository can't be opened or HEAD can't be resolved
pub fn get_mobile_secrets_head_sha1(mobile_secrets_path: &Path) -> Result<String> {
    let repo = git2::Repository::open(mobile_secrets_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Extracts the repository name from the current directory's git remote origin URL.
///
/// # Returns
/// - `Ok(String)` containing the repository name (e.g., "my-repo" from "git@github.com:user/my-repo.git")
/// - `Err(anyhow::Error)` if git repository can't be opened, origin remote doesn't exist, or URL is invalid
pub fn get_current_repo_name() -> Result<String> {
    let repo = git2::Repository::open(".")?;
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().ok_or_else(|| anyhow!("No remote URL found"))?;
    extract_repo_name(remote_url)
}

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

/// Extracts the repository name from a git remote URL.
///
/// Handles both SSH and HTTPS URLs, removing .git suffix if present.
///
/// # Arguments
/// - `url` - The git remote URL (e.g., "git@github.com:user/repo.git" or "https://github.com/user/repo")
///
/// # Returns
/// - `Ok(String)` containing the repository name (the last path component)
/// - `Err(anyhow::Error)` if the URL format is invalid or doesn't contain a repository name
fn extract_repo_name(url: &str) -> Result<String> {
    let url = url.trim_end_matches(".git");
    let name = url
        .split('/')
        .next_back()
        .ok_or_else(|| anyhow!("Could not extract repo name from URL"))?;
    Ok(name.to_owned())
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

/// Checks if the mobile-secrets repository is up-to-date with its remote.
///
/// This function checks if the local ~/.mobile-secrets repository is behind its remote
/// and provides a warning to the user if updates are available.
///
/// # Arguments
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(())` if the repository is up-to-date or the user chooses to continue
/// - `Err(anyhow::Error)` if the user chooses to abort or there's an error
pub fn check_mobile_secrets_up_to_date(mobile_secrets_path: &Path) -> Result<()> {
    let repo = git2::Repository::open(mobile_secrets_path)?;
    
    // Fetch the latest changes from remote
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["trunk"], None, None)?;
    
    // Get the current HEAD commit
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    
    // Get the remote trunk commit
    let trunk_ref = repo.find_reference("refs/remotes/origin/trunk")?;
    let trunk_commit = trunk_ref.peel_to_commit()?;
    
    // Check if we're behind the remote trunk
    let commits_behind = repo.graph_ahead_behind(head_commit.id(), trunk_commit.id())?.1;
    
    if commits_behind > 0 {
        println!("⚠️  Warning: Your ~/.mobile-secrets repository is {} commit(s) behind origin/trunk.", commits_behind);
        println!("   This means you might be encrypting outdated secrets.");
        println!();
        println!("   To update to the latest version, run:");
        println!("   cd ~/.mobile-secrets && git checkout trunk && git pull");
        println!();
        
        // Ask user if they want to continue
        println!("   Do you want to continue with the current version? (y/N)");
        
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                println!("   Continuing with current version...");
                return Ok(());
            } else {
                return Err(anyhow!(
                    "Update cancelled. Please update ~/.mobile-secrets first:\n\
                    cd ~/.mobile-secrets && git checkout trunk && git pull"
                ));
            }
        } else {
            return Err(anyhow!("Failed to read user input. Please update ~/.mobile-secrets first."));
        }
    }

    Ok(())
}

/// Ensures that a destination file path is ignored by git.
///
/// This function checks if the destination file would be ignored by git to prevent
/// accidentally committing decrypted secret files to version control.
///
/// # Arguments
/// - `destination_path` - The path to the destination file to validate
///
/// # Returns
/// - `Ok(())` if the file is properly ignored
/// - `Err(anyhow::Error)` if the file is not ignored or validation fails
pub fn ensure_destination_is_git_ignored(destination_path: &str) -> Result<()> {
    let repo = git2::Repository::open(".").map_err(|e| {
        anyhow!(
            "Failed to open git repository: {}\n\n\
            Make sure you're running this command from within a git repository.",
            e
        )
    })?;

    let is_ignored = repo.is_path_ignored(Path::new(destination_path)).map_err(|e| {
        anyhow!(
            "Failed to check if path '{}' is ignored by git: {}\n\n\
            This could indicate a problem with the git repository or the file path.",
            destination_path,
            e
        )
    })?;

    if is_ignored {
        // File is properly ignored, continue
        Ok(())
    } else {
        Err(anyhow!(
            "⚠️  SECURITY ERROR: Destination file '{}' is NOT ignored by git!\n\n\
            Refusing to create decrypted secret file that could be accidentally committed.\n\n\
            To fix this issue:\n\
            1. Add '{}' to your .gitignore file, OR\n\
            2. Change the destination path to a location outside your repository, OR\n\
            3. Change the destination path to a location that's already git-ignored\n\n\
            Example .gitignore entries:\n\
            # Ignore this specific file\n\
            {}\n\
            # Or ignore all secret files in a directory\n\
            secrets/\n\
            *.secret\n\n\
            After updating .gitignore, run 'git check-ignore {}' to verify it's ignored.",
            destination_path, destination_path, destination_path, destination_path
        ))
    }
}
