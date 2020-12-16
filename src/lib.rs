mod configure;
mod encryption;
mod fs;
mod git;
mod ui;

use crate::configure::*;
use crate::fs::*;
use log::debug;

/// Set up a project to use the configure tool
///
pub fn init() {
    init_encryption();
    let configuration = read_configuration();
    setup_configuration(configuration);
}

/// Decrypts secrets already present in the repository
///
/// To get secrets into the repository, use `configure_update`
///
/// # Arguments
///
/// * `configuration` - The project's parsed `ConfigurationFile` object.'
/// * `interactive` - Whether to prompt the user for confirmation before performing destructive operations
///
pub fn apply(interactive: bool) {
    init_encryption();
    let configuration = read_configuration();

    if configuration.is_empty() {
        if interactive {
            setup_configuration(configuration);
        } else {
            ui::warn("Unable to apply configuration – it is empty");
        }
    } else {
        apply_configuration(&configuration);
    }
}

/// Adds encrypted secrets files to the configuration, or updates existing ones.
///
/// Prompts the user to decrypt them when it finishes.
///
/// # Arguments
///
/// * `configuration` - The project's parsed `ConfigurationFile` object.
/// * `interactive` - Whether to prompt the user for confirmation before performing destructive operations
///
pub fn update(interactive: bool) {
    init_encryption();
    let configuration = read_configuration();

    if configuration.is_empty() {
        if interactive {
            setup_configuration(configuration)
        } else {
            ui::warn("Unable to update configuration – not running in interactive mode");
        }
    } else {
        update_configuration(configuration, interactive);
    }
}

/// Validate a project's .configure file
///
pub fn validate() {
    init_encryption();
    let configuration = read_configuration();

    if configuration.is_empty() {
        ui::warn("Unable to validate configuration – it is empty");
    } else {
        validate_configuration(configuration);
    }
}

/// Create an encryption key suitable for use with this project
///
pub fn generate_encryption_key() -> String {
    crate::encryption::generate_key()
}

fn init_encryption() {
    debug!("libConfigure initializing encryption");
    encryption::init();
    debug!("libConfigure encryption initialization successful");
}

const SECRETS_KEY_NAME: &str = "SECRETS_REPO";
