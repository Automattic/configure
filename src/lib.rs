mod configure;
mod encryption;
mod fs;
mod git;
mod string;
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
/// * `configuration` - The project's parsed `ConfigurationFile` object.
/// * `interactive` - Whether to prompt the user for confirmation before performing destructive operations
///
pub fn apply(interactive: bool, configuration_file_path: Option<String>) {
    init_encryption();
    let configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read configuration from `.configure` file");

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
/// * `interactive` - Whether to prompt the user for confirmation before performing destructive operations
/// * `configuration_file_path` - An optional path to the configuration file that should be updated. Useful for when the working directory differs from the root project directory (as when using the gradle plugin, for instance). If this value is `None`, the default configuration file path will be used.
///
pub fn update(interactive: bool, configuration_file_path: Option<String>) {
    init_encryption();

    let configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read configuration from `.configure` file");

    if configuration.is_empty() {
        if interactive {
            setup_configuration(configuration)
        } else {
            ui::warn("Current configuration is empty – unable to update when running in non-interactive mode");
        }
    } else {
        update_configuration(configuration_file_path, interactive);
    }
}

/// Update the project name in the project `.configure` file
///
/// # Arguments
///
/// * `project_name` – the new project name that should be written to the `.configure` file.
pub fn update_project_name(project_name: String, configuration_file_path: Option<String>) {
    let mut configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read project configuration");
    configuration.project_name = project_name;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Update the branch name in the project `.configure` file.
///
/// # Arguments
///
/// * `branch_name` – the new branch name read_configurationthat should be written to the `configure` file
pub fn update_branch_name(branch_name: String, configuration_file_path: Option<String>) {
    let mut configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read project configuration");
    configuration.branch = branch_name;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Update the pinned hash in the project `.configure` file
///
/// # Arguments
///
/// * `pinned_hash` – the commit hash to copy configuration files from
pub fn update_pinned_hash(pinned_hash: String, configuration_file_path: Option<String>) {
    let mut configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read project configuration");
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
/// The encryption key will be written to the `keys.json` file at the root of your local secrets repository. You will need to commit this change yourself.
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
const TEMP_ENCRYPTION_KEY_NAME: &str = "CONFIGURE_ENCRYPTION_KEY_TEMP"; // Useful when switching between versions of the plugin
