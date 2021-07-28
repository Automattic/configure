mod configure;
mod encryption;
mod fs;
mod git;
mod string;
mod ui;

use crate::configure::*;
use crate::encryption::EncryptionKey;
use crate::fs::*;

use libc::c_char;
use log::debug;
use std::ffi::CStr;
use std::path::Path;
use std::path::PathBuf;

/// Set up a project to use the configure tool
///
#[no_mangle]
pub extern "C" fn init() {
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

/// An FFI-compatible version of the `apply` function
///
/// # Safety
///
/// This function takes a C string as input and needs to handle deallocation of it. The function will panic
/// if the string is null.
#[export_name = "apply"]
pub unsafe extern "C" fn c_compatible_apply(
    interactive: bool,
    configuration_file_path: *const c_char,
) {
    let c_str = {
        assert!(!configuration_file_path.is_null());
        CStr::from_ptr(configuration_file_path)
    };

    let configuration_file_path = c_str.to_str().unwrap();
    apply(interactive, Some(configuration_file_path.to_string()))
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

/// An FFI-compatible version of the `update` function
///
/// # Safety
///
/// This function takes a C string as input and needs to handle deallocation of it. The function will panic
/// if the string is null.
#[export_name = "update"]
pub unsafe extern "C" fn c_compatible_update(
    interactive: bool,
    configuration_file_path: *const c_char,
) {
    let c_str = {
        assert!(!configuration_file_path.is_null());
        CStr::from_ptr(configuration_file_path)
    };

    let configuration_file_path = c_str.to_str().unwrap();
    update(interactive, Some(configuration_file_path.to_string()))
}

/// Update the project name in the project `.configure` file
///
/// # Arguments
///
/// * `project_name` – the new project name that should be written to the `.configure` file.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
pub fn update_pinned_hash(pinned_hash: String, configuration_file_path: Option<String>) {
    let mut configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read project configuration");
    configuration.pinned_hash = pinned_hash;
    write_configuration(&configuration).expect("Unable to save project configuration");
}

/// Validate a project's .configure file
///
#[no_mangle]
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
#[no_mangle]
pub fn generate_encryption_key() -> String {
    crate::encryption::generate_key().to_string()
}

/// Finds the `.configure` file in the current project and returns a string containing it.
#[no_mangle]
pub fn find_configuration_file() -> String {
    match fs::find_configure_file() {
        Ok(path) => match path.into_os_string().into_string() {
            Ok(string) => string,
            Err(_) => "".to_string(),
        },
        Err(_) => "".to_string(),
    }
}

/// A wrapper around `encrypt_single_file` that takes strings instead of paths.
///
/// This makes it easier to call from command line arguments.
pub fn encrypt_single_file_path(
    input_file: &str,
    output_file: Option<String>,
    encryption_key_string: Option<String>,
) {
    let input_file_path = Path::new(input_file).to_path_buf();
    let output_file_path = output_file.map(|path| Path::new(&path).to_path_buf());
    encrypt_single_file(input_file_path, output_file_path, encryption_key_string)
}

pub fn encrypt_single_file(
    input_file: PathBuf,
    output_file: Option<PathBuf>,
    encryption_key_string: Option<String>,
) {
    let encryption_key = match encryption_key_string {
        Some(encryption_key_string) => match EncryptionKey::from_str(&encryption_key_string) {
            Ok(encryption_key) => encryption_key,
            Err(err) => {
                println!("{:?}", err);
                std::process::exit(err as i32);
            }
        },
        None => {
            let key = crate::encryption::generate_key();
            println!("Using autogenerated key {:}.\n\nBe sure to save it somewhere right away – it won't be available again.", key);
            key
        }
    };

    // Infer the output path based on the input path if needed
    let output_file = match output_file {
        Some(path) => path,
        None => infer_encryption_output_filename(&input_file),
    };

    encryption::encrypt_file(
        Path::new(&input_file),
        Path::new(&output_file),
        &encryption_key,
    )
    .expect("Unable to encrypt file");
}

/// A wrapper around `encrypt_single_file` that takes strings instead of paths.
///
/// This makes it easier to call from command line arguments.
pub fn decrypt_single_file_path(
    input_file: &str,
    output_file: Option<String>,
    encryption_key_string: String,
) {
    let input_file_path = Path::new(input_file).to_path_buf();
    let output_file_path = output_file.map(|path| Path::new(&path).to_path_buf());

    decrypt_single_file(input_file_path, output_file_path, encryption_key_string)
}

#[no_mangle]
pub fn decrypt_single_file(
    input_file: PathBuf,
    output_file: Option<PathBuf>,
    encryption_key_string: String,
) {
    let encryption_key = match EncryptionKey::from_str(&encryption_key_string) {
        Ok(encryption_key) => encryption_key,
        Err(err) => {
            println!("{:?}", err);
            std::process::exit(err as i32);
        }
    };

    // Infer the file path based on the input path
    let output_file = match output_file {
        Some(path) => path,
        None => infer_decryption_output_filename(&input_file),
    };

    encryption::decrypt_file(
        Path::new(&input_file),
        Path::new(&output_file),
        &encryption_key,
    )
    .expect("Unable to decrypt file");
}

fn init_encryption() {
    debug!("libConfigure initializing encryption");
    encryption::init();
    debug!("libConfigure encryption initialization successful");
}

const SECRETS_KEY_NAME: &str = "SECRETS_REPO";
const ENCRYPTION_KEY_NAME: &str = "CONFIGURE_ENCRYPTION_KEY";
const TEMP_ENCRYPTION_KEY_NAME: &str = "CONFIGURE_ENCRYPTION_KEY_TEMP"; // Useful when switching between versions of the plugin

#[cfg(test)]
mod tests {
    // Import the parent scope
    use super::*;
    use std::path::PathBuf;

    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::env::temp_dir;

    use std::fs;

    #[test]
    fn test_that_single_file_encryption_works_end_to_end() {
        let random_string = __randomstring();
        let key = crate::encryption::generate_key().to_string();

        let input_file_path = __tempfile();
        let input_file_path_string = input_file_path.to_str().unwrap();

        let encrypted_file_path = __tempfile();
        let encrypted_file_path_string = encrypted_file_path.to_str().unwrap();

        let output_file_path = __tempfile();
        let output_file_path_string = output_file_path.to_str().unwrap();

        fs::write(&input_file_path, &random_string).unwrap();
        println!("{:?}", random_string);
        println!("{:?}", input_file_path_string);

        encrypt_single_file_path(
            input_file_path_string,
            Some(encrypted_file_path_string.to_string()),
            Some(key.clone()),
        );

        decrypt_single_file_path(
            encrypted_file_path_string,
            Some(output_file_path_string.to_string()),
            key.clone(),
        );

        let result = fs::read_to_string(&output_file_path).unwrap();

        assert_eq!(result, random_string);
    }

    fn __tempfile() -> PathBuf {
        let mut dir = temp_dir();
        let name: String = __randomstring();
        let file_name = format!("{}.txt", name);
        dir.push(file_name);

        dir
    }

    fn __randomstring() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }
}
