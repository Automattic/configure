use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use crate::{Config, REPO_SECRETS_CONFIG_FILE, REPO_SECRETS_DIR};

/// Validates a source path relative to ~/.mobile-secrets.
///
/// # Arguments
/// - `source_path` - The source path to validate
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(())` if the path is valid
/// - `Err(anyhow::Error)` if the path is invalid
pub fn validate_source_path(source_path: &str, mobile_secrets_path: &Path) -> Result<()> {
    // Check for empty path
    if source_path.trim().is_empty() {
        return Err(anyhow!("Source path cannot be empty"));
    }

    // Check for absolute paths (should be relative to ~/.mobile-secrets)
    if Path::new(source_path).is_absolute() {
        return Err(anyhow!(
            "Source path '{}' is absolute, but must be relative to ~/.mobile-secrets",
            source_path
        ));
    }

    // Check for path traversal attempts
    if source_path.contains("..") {
        return Err(anyhow!(
            "Source path '{}' contains '..' which is not allowed for security reasons",
            source_path
        ));
    }

    // Check if the file exists
    let full_source_path = mobile_secrets_path.join(source_path);
    if !full_source_path.exists() {
        return Err(anyhow!(
            "Source file '{}' does not exist at {}",
            source_path,
            full_source_path.display()
        ));
    }

    // Check if it's actually a file (not a directory)
    if !full_source_path.is_file() {
        return Err(anyhow!(
            "Source path '{}' points to a directory, but must point to a file",
            source_path
        ));
    }

    Ok(())
}

/// Gets the path to the ~/.mobile-secrets directory.
///
/// # Returns
/// - `Ok(PathBuf)` containing the path to ~/.mobile-secrets
/// - `Err(anyhow::Error)` if the HOME environment variable is not set
pub fn get_mobile_secrets_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| {
        anyhow!(
            "HOME environment variable not set.\n\n\
            This is required to locate the ~/.mobile-secrets directory.\n\
            Make sure you're running this command in a proper shell environment."
        )
    })?;
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

    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
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
