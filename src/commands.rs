use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::crypto::{
    decrypt_data, encrypt_data, generate_encryption_key, get_encryption_key_for_current_repo,
};
use crate::git::{
    check_mobile_secrets_up_to_date, ensure_destination_is_git_ignored, get_current_repo_name,
    get_mobile_secrets_head_sha1,
};
use crate::paths::{get_mobile_secrets_path, load_config};
use crate::{
    Config, ENV_VAR_KEY, MOBILE_SECRETS_ENCRYPTION_KEYS_FILE, REPO_SECRETS_CONFIG_FILE,
    REPO_SECRETS_DIR,
};

/// Sets up or validates secrets configuration for the current repository.
///
/// This function:
/// - If no setup exists: Creates configuration and generates encryption key
/// - If setup exists: Validates and displays current configuration
///
/// For initial setup:
/// - Creates a `.a8c-secrets/config.yaml` configuration file with the current SHA1 of ~/.mobile-secrets
/// - Generates a unique AES-256 encryption key for this repository
/// - Stores the key in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`
/// - Provides setup instructions for CI environments
///
/// For existing setup:
/// - Validates the configuration file structure
/// - Checks if encryption key exists
/// - Displays current setup status
///
/// # Returns
/// - `Ok(())` if setup/validation succeeds
/// - `Err(anyhow::Error)` if any step fails (git operations, file I/O, validation errors, etc.)
pub fn setup_command() -> Result<()> {
    let mobile_secrets_path = get_mobile_secrets_path()?;
    let config_path = std::path::Path::new(REPO_SECRETS_DIR).join(REPO_SECRETS_CONFIG_FILE);
    let config_exists = config_path.exists();

    // Check if encryption key already exists
    let repo_name = get_current_repo_name()?;
    let keys_file_path = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
    let key_exists = if keys_file_path.exists() {
        let keys_content = fs::read_to_string(&keys_file_path)?;
        let keys: HashMap<String, String> = serde_yaml::from_str(&keys_content).unwrap_or_default();
        keys.contains_key(&repo_name)
    } else {
        false
    };

    // If setup already exists, validate and display instead of creating
    if config_exists || key_exists {
        return validate_and_display_setup(
            config_exists,
            key_exists,
            &repo_name,
            &keys_file_path,
            &config_path,
        );
    }

    // Proceed with initial setup
    perform_initial_setup(&mobile_secrets_path, &repo_name)
}

/// Performs the initial setup when no existing configuration is found.
pub fn perform_initial_setup(mobile_secrets_path: &std::path::Path, repo_name: &str) -> Result<()> {
    println!("🚀 Setting up secrets configuration for the first time...");
    println!();

    let sha1 = get_mobile_secrets_head_sha1(mobile_secrets_path)?;

    // Create initial config
    let config = Config {
        sha1,
        files: Vec::new(),
    };

    // Create .a8c-secrets directory
    fs::create_dir_all(REPO_SECRETS_DIR)?;

    // Write config file with examples
    let config_yaml_with_examples = format!(
        r#"sha1: {}
files: []
# Example entries - edit this section to specify which secret files to sync:
# files:
#   - source: "path/to/secrets.properties" # Path relative to ~/.mobile-secrets
#     destination: "config/secret1.json"   # Path relative to current repository
#     # Prefer decrypting files outside of your repository if possible, so that you don't risk committing them
#     # by accident but also don't risk them being scanned by LLMs you might use on your machine.
#   - source: "shared/Secrets.swift"
#     destination: "~/.a8c-secrets/myapp/Secrets.swift"
"#,
        config.sha1
    );
    let config_path = Path::new(REPO_SECRETS_DIR).join(REPO_SECRETS_CONFIG_FILE);
    fs::write(&config_path, config_yaml_with_examples)?;

    // Generate encryption key
    let key = generate_encryption_key();
    let key_b64 = general_purpose::STANDARD.encode(key);

    // Update encryption keys file
    let keys_file_path = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
    let mut keys: HashMap<String, String> = if keys_file_path.exists() {
        let keys_content = fs::read_to_string(&keys_file_path)?;
        serde_yaml::from_str(&keys_content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    keys.insert(repo_name.to_owned(), key_b64.clone());
    let keys_yaml = serde_yaml::to_string(&keys)?;
    fs::write(&keys_file_path, keys_yaml)?;

    println!(
        "✅ Configuration file '{}' created successfully.",
        config_path.display()
    );
    println!(
        "✅ Encryption key generated and saved to {}",
        keys_file_path.display()
    );
    println!("🎉 Setup complete!");
    println!("📋 Next steps:");
    println!("1. Add the following environment variable to ~/.mobile-secrets/CI/secrets/{repo_name}/env for CI:");
    println!();
    println!("   export {ENV_VAR_KEY}=\"{key_b64}\"");
    println!();
    println!("2. Commit and push the changes to `~/.mobile-secrets`'s `trunk` branch directly.");
    println!("   (Both the manual changes you made to the `env` file and the change the setup script made of the `{MOBILE_SECRETS_ENCRYPTION_KEYS_FILE}` file)");
    println!(
        "3. Edit {} to specify which secret files you want to sync for your repository.",
        config_path.display()
    );
    println!("4. Run `a8c-secrets encrypt` to encrypt those secrets files into your repository.");

    Ok(())
}

/// Validates existing setup and displays current configuration.
pub fn validate_and_display_setup(
    config_exists: bool,
    key_exists: bool,
    repo_name: &str,
    keys_file_path: &std::path::Path,
    config_path: &std::path::Path,
) -> Result<()> {
    println!("🔍 Existing setup detected - validating configuration...");
    println!();

    // Validate and display config file
    if config_exists {
        match load_config() {
            Ok(config) => {
                println!("✅ Configuration file: {}", config_path.display());
                println!("   📋 SHA1: {}", config.sha1);
                println!("   📁 Secret files configured: {}", config.files.len());
                if config.files.is_empty() {
                    println!(
                        "   ⚠️  No secret files configured yet - edit {} to add them",
                        config_path.display()
                    );
                } else {
                    for (i, file) in config.files.iter().enumerate() {
                        println!("   {}. {} → {}", i + 1, file.source, file.destination);
                    }
                }
            }
            Err(e) => {
                println!("❌ Configuration file: {}", config_path.display());
                println!("   Error: Invalid YAML structure - {e}");
                return Err(anyhow!("Configuration file validation failed: {}", e));
            }
        }
    } else {
        println!("❌ Configuration file: {} (missing)", config_path.display());
    }

    println!();

    // Display encryption key status
    if key_exists {
        println!("✅ Encryption key: Found for repository '{repo_name}'");
        println!("   📍 Location: {}", keys_file_path.display());
    } else {
        println!("❌ Encryption key: Not found for repository '{repo_name}'");
        if keys_file_path.exists() {
            println!("   📍 Keys file exists at: {}", keys_file_path.display());
            println!("   ⚠️  But no key found for this repository");
        } else {
            println!("   📍 Keys file missing: {}", keys_file_path.display());
        }
    }

    println!();

    // Summary and recommendations
    match (config_exists, key_exists) {
        (true, true) => {
            println!("🎉 Setup is complete and valid!");
            println!("   You can now run 'encrypt' to encrypt secrets or 'decrypt' to decrypt them.");
        }
        (true, false) => {
            println!(
                "⚠️  Setup is incomplete: Configuration exists but encryption key is missing."
            );
            println!("   Please run this command in a fresh directory to generate a new key,");
            println!("   or manually add the key to {}", keys_file_path.display());
        }
        (false, true) => {
            println!(
                "⚠️  Setup is incomplete: Encryption key exists but configuration is missing."
            );
            println!(
                "   Please run this command in a fresh directory to create the configuration."
            );
        }
        (false, false) => unreachable!("This case is handled by initial setup"),
    }

    Ok(())
}

/// Encrypts secrets from ~/.mobile-secrets into the current repository.
///
/// This function:
/// - Loads the current `.a8c-secrets/config.yaml` configuration
/// - Updates the SHA1 to match the current HEAD of ~/.mobile-secrets
/// - Encrypts each configured secret file using the repository's unique key
/// - Saves encrypted files as `.a8c-secrets/*.enc` binary files
///
/// # Returns
/// - `Ok(())` if all secrets are successfully encrypted and saved
/// - `Err(anyhow::Error)` if configuration loading, encryption, or file I/O fails
pub fn encrypt_command() -> Result<()> {
    let mobile_secrets_path = get_mobile_secrets_path()?;
    
    // Check if mobile-secrets repository is up-to-date
    check_mobile_secrets_up_to_date(&mobile_secrets_path)?;
    
    let mut config = load_config()?;
    let key = get_encryption_key_for_current_repo()?;

    // Update SHA1 to current HEAD of mobile-secrets repo
    config.sha1 = get_mobile_secrets_head_sha1(&mobile_secrets_path)?;

    // Write updated config back to file
    let config_yaml = serde_yaml::to_string(&config)?;
    let config_path = Path::new(REPO_SECRETS_DIR).join(REPO_SECRETS_CONFIG_FILE);
    fs::write(&config_path, config_yaml)?;

    // Create secrets directory (should already exist, but ensure it does)
    fs::create_dir_all(REPO_SECRETS_DIR)?;

    if config.files.is_empty() {
        println!("⚠️  No secret files configured for encryption.");
        println!(
            "Edit {} to add files to sync.",
            Path::new(REPO_SECRETS_DIR)
                .join(REPO_SECRETS_CONFIG_FILE)
                .display()
        );
        return Ok(());
    }

    for file_config in &config.files {
        let source_path = mobile_secrets_path.join(&file_config.source);

        if !source_path.exists() {
            return Err(anyhow!(
                "Source file not found: {}\n\n\
                Expected location: {}\n\
                Configured in: {}\n\n\
                Please check that:\n\
                1. The file exists in the ~/.mobile-secrets repository\n\
                2. The source path '{}' is correct relative to ~/.mobile-secrets\n\
                3. You have read permissions for the file",
                file_config.source,
                source_path.display(),
                Path::new(REPO_SECRETS_DIR)
                    .join(REPO_SECRETS_CONFIG_FILE)
                    .display(),
                file_config.source
            ));
        }

        // Check if the destination file would be ignored by git
        ensure_destination_is_git_ignored(&file_config.destination)?;

        let content = fs::read(&source_path).map_err(|e| {
            anyhow!(
                "Failed to read source file {}: {}\n\n\
                Please check that you have read permissions for the file.",
                source_path.display(),
                e
            )
        })?;

        let encrypted = encrypt_data(&content, &key)?;

        let dest_filename = Path::new(&file_config.source)
            .file_name()
            .ok_or_else(|| {
                anyhow!(
                    "Invalid source path '{}': cannot extract filename.\n\n\
                The source path must point to a file, not a directory.",
                    file_config.source
                )
            })?
            .to_string_lossy();
        let encrypted_path = format!("{REPO_SECRETS_DIR}/{dest_filename}.enc");

        fs::write(&encrypted_path, encrypted).map_err(|e| {
            anyhow!(
                "Failed to write encrypted file {}: {}\n\n\
                Please check that you have write permissions in the current directory.",
                encrypted_path,
                e
            )
        })?;

        println!("Encrypted {} -> {}", file_config.source, encrypted_path);
    }

    println!("✅ Encryption of secrets files from `~/.mobile-secrets` into `.enc` files in your repository is complete!");
    println!("✅ You can now commit and push the changes to the `.enc` files in your repository,");
    println!("   and run `a8c-secrets decrypt` to decrypt them to their configured local destination.");

    Ok(())
}

/// Decrypts secrets from .a8c-secrets/*.enc files to their configured destinations.
///
/// This function:
/// - Loads the `.a8c-secrets/config.yaml` configuration
/// - Obtains the decryption key from environment variable or ~/.mobile-secrets
/// - Decrypts each `.a8c-secrets/*.enc` file
/// - Writes decrypted content to the destination paths specified in config
/// - Creates destination directories as needed
///
/// # Returns
/// - `Ok(())` if all secrets are successfully decrypted and written
/// - `Err(anyhow::Error)` if key retrieval, decryption, or file I/O fails
pub fn decrypt_command() -> Result<()> {
    let config = load_config()?;
    let key = get_encryption_key_for_current_repo()?;

    if config.files.is_empty() {
        println!("⚠️  No secret files configured for decryption.");
        println!(
            "Edit {} to add files to sync.",
            Path::new(REPO_SECRETS_DIR)
                .join(REPO_SECRETS_CONFIG_FILE)
                .display()
        );
        return Ok(());
    }

    for file_config in &config.files {
        // Check if the destination file would be ignored by git (early validation)
        ensure_destination_is_git_ignored(&file_config.destination)?;

        let source_filename = Path::new(&file_config.source)
            .file_name()
            .ok_or_else(|| {
                anyhow!(
                    "Invalid source path '{}': cannot extract filename.\n\n\
                The source path must point to a file, not a directory.",
                    file_config.source
                )
            })?
            .to_string_lossy();
        let encrypted_path = format!("{REPO_SECRETS_DIR}/{source_filename}.enc");

        if !Path::new(&encrypted_path).exists() {
            return Err(anyhow!(
                "Encrypted file not found: {}\n\n\
                Expected location: {}\n\
                Source configured as: {}\n\
                Destination configured as: {}\n\n\
                This usually means:\n\
                1. The encrypted file hasn't been created yet - run 'a8c-secrets encrypt' first\n\
                2. The source filename in the configuration is incorrect\n\
                3. The encrypted file was manually deleted",
                source_filename,
                encrypted_path,
                file_config.source,
                file_config.destination
            ));
        }

        let encrypted = fs::read(&encrypted_path).map_err(|e| {
            anyhow!(
                "Failed to read encrypted file {}: {}\n\n\
                Please check that you have read permissions for the file.",
                encrypted_path,
                e
            )
        })?;

        let decrypted = decrypt_data(&encrypted, &key).map_err(|e| {
            anyhow!(
                "Failed to decrypt file {}: {}\n\n\
                This could indicate:\n\
                1. The file was corrupted\n\
                2. Wrong encryption key for this repository\n\
                3. The file was not encrypted with a8c-secrets",
                encrypted_path,
                e
            )
        })?;

        // Ensure destination directory exists
        if let Some(parent) = Path::new(&file_config.destination).parent() {
            fs::create_dir_all(parent).map_err(|e| {
                anyhow!(
                    "Failed to create destination directory {}: {}\n\n\
                    Please check that you have write permissions.",
                    parent.display(),
                    e
                )
            })?;
        }

        fs::write(&file_config.destination, decrypted).map_err(|e| {
            anyhow!(
                "Failed to write decrypted file {}: {}\n\n\
                Please check that you have write permissions for the destination.",
                file_config.destination,
                e
            )
        })?;

        println!(
            "Decrypted {} -> {}",
            encrypted_path, file_config.destination
        );
    }

    println!("✅ Decryption of `.enc` secrets files to their configured destinations is complete!");

    Ok(())
}
