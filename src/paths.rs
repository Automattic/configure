use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use crate::{Config, REPO_SECRETS_CONFIG_FILE, REPO_SECRETS_DIR};

/// Expands a path that may contain a tilde (~) to the user's home directory.
///
/// # Arguments
/// - `path` - The path that may contain a tilde
///
/// # Returns
/// - `PathBuf` with the tilde expanded to the user's home directory if present
/// - The original path unchanged if no tilde is present
pub fn expand_tilde_path(path: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);
    let mut components = path_buf.components();

    if let Some(first_component) = components.next() {
        if first_component == std::path::Component::Normal(std::ffi::OsStr::new("~")) {
            let home = std::env::var("HOME").map_err(|_| {
                anyhow!(
                    "HOME environment variable not set.\n\n\
                    This is required to expand tilde (~) in path '{}'.\n\
                    Make sure you're running this command in a proper shell environment.",
                    path
                )
            })?;

            let home_path = PathBuf::from(home);
            let remaining_components: Vec<_> = components.collect();

            if remaining_components.is_empty() {
                Ok(home_path)
            } else {
                Ok(home_path.join(remaining_components.into_iter().collect::<PathBuf>()))
            }
        } else {
            Ok(path_buf)
        }
    } else {
        Ok(path_buf)
    }
}

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

/// Loads the `.a8c-secrets/config.yaml` configuration file from the specified repository directory.
///
/// # Arguments
/// - `repo_path` - Path to the repository directory containing the .a8c-secrets configuration
///
/// # Returns
/// - `Ok(Config)` containing the parsed configuration
/// - `Err(anyhow::Error)` if the file doesn't exist, can't be read, or contains invalid YAML
pub fn load_config(repo_path: &Path) -> Result<Config> {
    let config_path = repo_path
        .join(REPO_SECRETS_DIR)
        .join(REPO_SECRETS_CONFIG_FILE);

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

    // Expand tilde paths in destination fields using functional approach
    let config = Config {
        files: config
            .files
            .into_iter()
            .map(|mut file_entry| {
                let expanded_path = expand_tilde_path(&file_entry.destination)?;
                file_entry.destination = expanded_path
                    .to_str()
                    .ok_or_else(|| {
                        anyhow!(
                            "Destination path '{}' contains invalid UTF-8 characters",
                            file_entry.destination
                        )
                    })?
                    .to_string();
                Ok(file_entry)
            })
            .collect::<Result<Vec<_>>>()?,
        ..config
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_mobile_secrets_path() {
        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Test with HOME set
        let temp_dir = tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        let path = get_mobile_secrets_path().unwrap();
        assert_eq!(path, temp_dir.path().join(".mobile-secrets"));

        // Restore original HOME value
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_get_mobile_secrets_path_no_home() {
        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Test without HOME set
        std::env::remove_var("HOME");

        let result = get_mobile_secrets_path();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HOME environment variable not set"));

        // Restore original HOME value
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_load_config() {
        let temp_dir = tempdir().unwrap();

        // Create .a8c-secrets directory
        let config_dir = temp_dir.path().join(".a8c-secrets");
        fs::create_dir_all(&config_dir).unwrap();

        // Test with valid YAML - ensure proper indentation and structure
        let valid_config = r#"sha1: "abc123def456"
files:
  - source: "secrets/api_key.txt"
    destination: "../api_key.txt"
  - source: "secrets/database.yml"
    destination: "config/database.yml"
"#;
        let config_path = config_dir.join("config.yaml");
        fs::write(&config_path, valid_config).unwrap();

        let result = load_config(temp_dir.path());
        let config = result.unwrap();
        assert_eq!(config.files.len(), 2);
        assert_eq!(config.files[0].source, "secrets/api_key.txt");
        assert_eq!(config.files[0].destination, "../api_key.txt");
        assert_eq!(config.files[1].source, "secrets/database.yml");
        assert_eq!(config.files[1].destination, "config/database.yml");
    }

    #[test]
    fn test_load_config_empty_file() {
        let temp_dir = tempdir().unwrap();

        // Create .a8c-secrets directory
        let config_dir = temp_dir.path().join(".a8c-secrets");
        fs::create_dir_all(&config_dir).unwrap();

        // Test with empty file - should fail because it's missing required fields
        let config_path = config_dir.join("config.yaml");
        fs::write(&config_path, "").unwrap();

        let result = load_config(temp_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should fail due to missing required fields or file not found
        assert!(
            error_msg.contains("missing field")
                || error_msg.contains("Invalid YAML")
                || error_msg.contains("Configuration file not found")
                || error_msg.contains("not been set up")
        );
    }

    #[test]
    fn test_load_config_invalid_yaml() {
        let temp_dir = tempdir().unwrap();

        // Create .a8c-secrets directory
        let config_dir = temp_dir.path().join(".a8c-secrets");
        fs::create_dir_all(&config_dir).unwrap();

        // Test with definitely invalid YAML
        let invalid_config = r#"sha1: "abc123def456"
files:
  - source: "secrets/api_key.txt"
    destination: "../api_key.txt"
  - source: "secrets/database.yml"
    destination: "config/database.yml"
    invalid_field: [unclosed_bracket
    another_invalid: "missing_quote
"#;
        let config_path = config_dir.join("config.yaml");
        fs::write(&config_path, invalid_config).unwrap();

        let result = load_config(temp_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Accept any error about invalid YAML or parsing failure
        assert!(
            error_msg.contains("YAML")
                || error_msg.contains("yaml")
                || error_msg.contains("Invalid")
                || error_msg.contains("parse")
                || error_msg.contains("syntax")
        );
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        // Create a fresh temp directory to ensure no config file exists
        let temp_dir = tempdir().unwrap();

        // This should fail because we're not in a directory with .a8c-secrets/config.yaml
        let result = load_config(temp_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Accept any error about missing configuration file
        assert!(
            error_msg.contains("Configuration file not found")
                || error_msg.contains("not found")
                || error_msg.contains("not been set up")
        );
    }

    #[test]
    fn test_validate_source_path_valid() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        // Create a test file
        let test_file = mobile_secrets_path.join("secrets").join("test.txt");
        fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        fs::write(&test_file, "test content").unwrap();

        let result = validate_source_path("secrets/test.txt", mobile_secrets_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_source_path_empty() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        let result = validate_source_path("", mobile_secrets_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Source path cannot be empty"));
    }

    #[test]
    fn test_validate_source_path_absolute() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        let result = validate_source_path("/absolute/path", mobile_secrets_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("is absolute, but must be relative"));
    }

    #[test]
    fn test_validate_source_path_traversal() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        let result = validate_source_path("secrets/../config.txt", mobile_secrets_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("contains '..' which is not allowed"));
    }

    #[test]
    fn test_validate_source_path_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        let result = validate_source_path("nonexistent.txt", mobile_secrets_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_source_path_directory() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        // Create a directory
        let test_dir = mobile_secrets_path.join("secrets");
        fs::create_dir_all(&test_dir).unwrap();

        let result = validate_source_path("secrets", mobile_secrets_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("points to a directory, but must point to a file"));
    }

    #[test]
    fn test_validate_source_path_special_characters() {
        let temp_dir = tempdir().unwrap();
        let mobile_secrets_path = temp_dir.path();

        // Test various special characters and whitespace that could cause issues in shells
        // Note: We only use characters that are valid in filenames on most filesystems
        let test_cases = vec![
            "file with spaces and\tmixed\twhitespace\ncharacters.txt",
            "file_with$dollar&pipe|semicolon;operators.txt",
            "file_with*asterisk?question[array]{braces}.txt",
            "file_with\"double\"'single'`backticks`quotes.txt",
            "file_with<less>greater=equals.txt",
            "file_with#hash@at!bang~tilde^caret%percent+plus-minus.txt",
            "file_with_underscore.dot,comma.txt",
        ];

        for filename in test_cases {
            // Try to create the test file with the special filename
            let test_file = mobile_secrets_path.join(filename);
            match fs::write(&test_file, "test content") {
                Ok(()) => {
                    // Test that the validation works with the special character filename
                    let result = validate_source_path(filename, mobile_secrets_path);
                    assert!(
                        result.is_ok(),
                        "Failed to validate path '{filename}': {result:?}"
                    );

                    // Clean up the test file
                    let _ = fs::remove_file(&test_file);
                }
                Err(e) => {
                    // Skip this test case if the filename is not supported by the filesystem
                    eprintln!(
                        "Skipping test case '{filename}' - filesystem doesn't support this filename: {e}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_load_config_with_tilde_destination() {
        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Test with HOME set
        let temp_dir = tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        // Create .a8c-secrets directory
        let config_dir = temp_dir.path().join(".a8c-secrets");
        fs::create_dir_all(&config_dir).unwrap();

        // Test with tilde in destination path
        let config_with_tilde = r#"sha1: "abc123def456"
files:
  - source: "secrets/api_key.txt"
    destination: "~/.my-app/secrets.swift"
"#;
        let config_path = config_dir.join("config.yaml");
        fs::write(&config_path, config_with_tilde).unwrap();

        let result = load_config(temp_dir.path());
        let config = result.unwrap();
        assert_eq!(config.files.len(), 1);
        assert_eq!(config.files[0].source, "secrets/api_key.txt");
        // The tilde should be expanded during load_config
        assert!(!config.files[0].destination.starts_with('~'));
        assert!(config.files[0].destination.contains(".my-app"));

        // Restore original HOME value
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_expand_tilde_path() {
        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Test with HOME set
        let temp_dir = tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        // Test tilde expansion
        let result = expand_tilde_path("~/.my-app/secrets.swift").unwrap();
        assert_eq!(
            result,
            temp_dir.path().join(".my-app").join("secrets.swift")
        );

        // Test just tilde
        let result = expand_tilde_path("~").unwrap();
        assert_eq!(result, temp_dir.path());

        // Test path without tilde
        let result = expand_tilde_path("config/secrets.env").unwrap();
        assert_eq!(result, std::path::PathBuf::from("config/secrets.env"));

        // Test absolute path without tilde
        let result = expand_tilde_path("/absolute/path").unwrap();
        assert_eq!(result, std::path::PathBuf::from("/absolute/path"));

        // Restore original HOME value
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_expand_tilde_path_no_home() {
        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Test without HOME set
        std::env::remove_var("HOME");

        let result = expand_tilde_path("~/.my-app/secrets.swift");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HOME environment variable not set"));

        // Restore original HOME value
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }
}
