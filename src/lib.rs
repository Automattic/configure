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
    let configuration =
        read_configuration().expect("Unable to read configuration from `.configure` file");
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
    let configuration =
        read_configuration().expect("Unable to read configuration from `.configure` file");

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
    let configuration =
        read_configuration().expect("Unable to read configuration from `.configure` file");

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

/// Update the project name in the project `.configure` file
///
/// # Arguments
///
/// * `project_name` – the new project name that should be written to the `.configure` file.
pub fn update_project_name(project_name: String) {
    let mut configuration = read_configuration().expect("Unable to read project configuration");
    configuration.project_name = project_name;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Update the branch name in the project `.configure` file.
///
/// # Arguments
///
/// * `branch_name` – the new branch name that should be written to the `configure` file
pub fn update_branch_name(branch_name: String) {
    let mut configuration = read_configuration().expect("Unable to read project configuration");
    configuration.branch = branch_name;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Update the pinned hash in the project `.configure` file
///
/// # Arguments
///
/// * `pinned_hash` – the commit hash to copy configuration files from
pub fn update_pinned_hash(pinned_hash: String) {
    let mut configuration = read_configuration().expect("Unable to read project configuration");
    configuration.pinned_hash = pinned_hash;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Validate a project's .configure file
///
pub fn validate() {
    init_encryption();
    let configuration =
        read_configuration().expect("Unable to read configuration from `.configure` file");

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
const ENCRYPTION_KEY_NAME: &str = "CONFIGURE_ENCRYPTION_KEY";
