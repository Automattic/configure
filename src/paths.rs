use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use crate::{Config, REPO_SECRETS_CONFIG_FILE, REPO_SECRETS_DIR};

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
