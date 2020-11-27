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
/// * `configuration` - The project's parsed `ConfigurationFile` object.
///
pub fn apply() {
    init_encryption();
    let configuration = read_configuration();

    if !configuration.is_empty() {
        apply_configuration(configuration);
    } else {
        setup_configuration(configuration);
    }
}

/// Adds encrypted secrets files to the configuration, or updates existing ones.
///
/// Prompts the user to decrypt them when it finishes.
///
/// # Arguments
///
/// * `configuration` - The project's parsed `ConfigurationFile` object.
///
pub fn update() {
    init_encryption();
    let configuration = read_configuration();

    if !configuration.is_empty() {
        update_configuration(configuration);
    } else {
        setup_configuration(configuration);
    }
}

/// Validate a project's .configure file
///
pub fn validate() {
    init_encryption();
    let configuration = read_configuration();

    if !configuration.is_empty() {
        validate_configuration(configuration);
    } else {
        setup_configuration(configuration);
    }
}

pub fn generate_encryption_key() -> String {
    crate::encryption::generate_key()
}

fn init_encryption() {
    debug!("libConfigure initializing encryption");
    encryption::init();
    debug!("libConfigure encryption initialization successful");
}
