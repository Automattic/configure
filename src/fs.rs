use crate::encryption::{decrypt_file, encrypt_file, generate_key};
use crate::Configuration;
use crate::ConfigureError;
use crate::EncryptionKey;
use log::{debug, info};
use ring::digest::{Context, SHA256};

use std::collections::HashMap;
use std::env;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{BufReader, Error, Read, Write};
use std::path::Path;
use std::path::PathBuf;

/// Find the .configure file in the current project
pub fn find_configure_file() -> Result<PathBuf, ConfigureError> {
    let configure_file_path = get_configure_file_path()?;

    if !configure_file_path.exists() {
        info!(
            "No configure file found at: {:?}. Creating one for you",
            configure_file_path
        );

        write_configuration_to(&Configuration::default(), &configure_file_path)?
    }

    debug!("Configure file found at: {:?}", configure_file_path);

    Ok(configure_file_path)
}

fn get_configure_file_path() -> Result<PathBuf, ConfigureError> {
    let project_root = find_project_root()?;
    Ok(project_root.join(".configure"))
}

pub fn find_keys_file() -> Result<PathBuf, ConfigureError> {
    let secrets_root = find_secrets_repo();
    let keys_file_path = secrets_root?.join("keys.json");

    debug!("Keys file found at: {:?}", keys_file_path);

    if !keys_file_path.exists() {
        info!(
            "No keys file found at: {:?}. Creating one for you",
            keys_file_path
        );

        let empty_keys: HashMap<String, String> = Default::default();
        save_keys(&keys_file_path, &empty_keys)?;
    }

    Ok(keys_file_path)
}

pub fn find_project_root() -> Result<PathBuf, ConfigureError> {
    let path = env::current_dir().expect("Unable to determine current directory");

    let repo = match git2::Repository::discover(&path) {
        Ok(repo) => repo,
        Err(_) => return Err(ConfigureError::ProjectNotPresent),
    };

    debug!("Discovered Repository at {:?}", &path);

    let project_root = match repo.workdir() {
        Some(dir) => dir,
        None => return Err(ConfigureError::ProjectNotPresent),
    };

    Ok(project_root.to_path_buf())
}

pub fn find_secrets_repo() -> Result<PathBuf, ConfigureError> {
    // Allow developers to specify where they want the secrets repo to be located using an environment variable
    if let Ok(var) = env::var(crate::SECRETS_KEY_NAME) {
        let user_secrets_path = Path::new(&var);

        if user_secrets_path.exists() && user_secrets_path.is_dir() {
            return Ok(user_secrets_path.to_path_buf());
        }
    }

    let home_dir = dirs::home_dir().expect("Unable to determine user home directory");
    let root_secrets_path = home_dir.join(".mobile-secrets");

    if root_secrets_path.exists() && root_secrets_path.is_dir() {
        return Ok(root_secrets_path);
    }

    // If the user has a `Projects` directory
    let projects_path = home_dir.join("Projects");
    if projects_path.exists() {
        let projects_secrets_path = projects_path.join(".mobile-secrets");
        if projects_secrets_path.exists() && projects_secrets_path.is_dir() {
            return Ok(projects_secrets_path);
        }
    }

    Err(crate::configure::ConfigureError::SecretsNotPresent)
}

pub fn read_configuration() -> Result<Configuration, ConfigureError> {
    read_configuration_from_file(&None)
}

pub fn resolve_configure_file_path(
    configure_file_path: &Option<String>,
) -> Result<PathBuf, ConfigureError> {
    match configure_file_path {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(find_configure_file()?),
    }
}

pub fn read_configuration_from_file(
    configure_file_path: &Option<String>,
) -> Result<Configuration, ConfigureError> {
    let configure_file_path = resolve_configure_file_path(configure_file_path)?;

    if !configure_file_path.is_file() {
        return Err(ConfigureError::ConfigureFileNotReadable);
    }

    let mut file = match File::open(&configure_file_path) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::ConfigureFileNotReadable),
    };

    let mut file_contents = String::new();
    match file.read_to_string(&mut file_contents) {
        Ok(_) => (), // no-op
        Err(_) => return Err(ConfigureError::ConfigureFileNotReadable),
    };

    Configuration::from_str(file_contents)
}

pub fn write_configuration(configuration: &Configuration) -> Result<(), ConfigureError> {
    let configuration_file = find_configure_file()?;
    write_configuration_to(configuration, &configuration_file)
}

pub fn write_configuration_to(
    configuration: &Configuration,
    configure_file: &Path,
) -> Result<(), ConfigureError> {
    let serialized = configuration.to_string()?;

    debug!("Writing to: {:?}", configure_file);

    let mut file = match File::create(configure_file) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::ConfigureFileNotWritable),
    };

    match file.write_all(serialized.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConfigureError::ConfigureFileNotWritable),
    }
}

pub fn generate_encryption_key_if_needed(
    configuration: &Configuration,
) -> Result<(), ConfigureError> {
    if encryption_key_for_configuration(configuration).is_ok() {
        return Ok(());
    }

    let keys_file_path = find_keys_file()?;

    let mut keys = read_keys(&keys_file_path)?;
    keys.insert(
        configuration.project_name.to_string(),
        generate_key().to_string(),
    );

    save_keys(&keys_file_path, &keys)
}

pub fn encryption_key_for_configuration(
    configuration: &Configuration,
) -> Result<EncryptionKey, ConfigureError> {
    let keys_file_path = find_keys_file()?;

    debug!("Reading keys from {:?}", keys_file_path);

    let keys = read_keys(&keys_file_path)?;

    // This is the first key that matches in the `keys.json` file
    let key = match keys.get(&configuration.project_name) {
        Some(key) => key,
        None => return Err(ConfigureError::MissingProjectKey),
    };

    EncryptionKey::from_str(key)
}

fn read_keys(source: &Path) -> Result<HashMap<String, String>, ConfigureError> {
    let file = match File::open(&source) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::KeysFileNotReadable),
    };

    let map: HashMap<String, String> = match serde_json::from_reader(file) {
        Ok(map) => map,
        Err(_) => return Err(ConfigureError::KeysFileIsNotValid),
    };

    Ok(map)
}

fn save_keys(destination: &Path, keys: &HashMap<String, String>) -> Result<(), ConfigureError> {
    let json = match serde_json::to_string_pretty(&keys) {
        Ok(json) => json,
        Err(_) => return Err(ConfigureError::KeysDataIsNotValid),
    };

    let mut file = match File::create(destination) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::KeysFileNotWritable),
    };

    match file.write_all(json.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConfigureError::KeysFileNotWritable),
    }
}

pub fn decrypt_files_for_configuration(
    configuration: &Configuration,
) -> Result<(), ConfigureError> {
    let project_root = find_project_root()?;

    let encryption_key: EncryptionKey;

    // Allow defining an environment variable that can override the key selection (for use in CI, for example).
    // This is placed here and not resued when encrypting files because it is a security risk to allow this override for
    // encryption – someone might set the encryption key on their local machine, causing every project to silently use the same key.
    //
    // We also have two sets of environment variables we accept – this makes it easier to transition between versions of the `configure` tool in production.
    // We check the temporary variable first, because it should override the permanent one when both are present
    if let Ok(var) = env::var(crate::TEMP_ENCRYPTION_KEY_NAME) {
        println!(
            "Found an environment variable named {:}. Using its value as the encryption key",
            crate::TEMP_ENCRYPTION_KEY_NAME
        );
        encryption_key = EncryptionKey::from_str(&var)?;
    } else if let Ok(var) = env::var(crate::ENCRYPTION_KEY_NAME) {
        println!(
            "Found an environment variable named {:}. Using its value as the encryption key",
            crate::ENCRYPTION_KEY_NAME
        );
        encryption_key = EncryptionKey::from_str(&var)?;
    } else if let Ok(var) = encryption_key_for_configuration(configuration) {
        encryption_key = var;
    } else {
        return Err(ConfigureError::MissingDecryptionKey);
    }

    for file in &configuration.files_to_copy {
        let source = project_root.join(&file.get_encrypted_destination());
        let destination = project_root.join(&file.get_decrypted_destination());

        create_parent_directory_for_path_if_not_exists(&destination)?;

        // If the developer tries to run `configure_apply` while missing the encrypted originals, this script will crash saying "missing file"
        // We can try to detect this scenario and fix things for the developer if the mobile secrets are available locally, but it's tricky because
        // we'd need to basically run `configure update` inside this method for just the one file. For now, we'll just error out.
        if !source.exists() {
            info!("Encrypted original file at {:?} not found", source);
            return Err(ConfigureError::EncryptedFileMissing {});
        }

        // If the file already exists, make a backup of the old one in case we need it later
        if destination.exists() {
            let backup_destination = project_root.join(&file.get_backup_destination());

            debug!(
                "{:?} already exists – making a backup at {:?}",
                destination, backup_destination
            );
            rename(&destination, &backup_destination)?;

            // Encrypt the file and write the encrypted contents to the destination
            debug!(
                "Encrypting file at {:?} and storing contents at {:?}",
                source, destination
            );
            decrypt_file(&source, &destination, &encryption_key)?;

            // If the backup file is identical to the old file, remove the backup
            let new_file_hash = hash_file(&destination);
            let original_file_hash = hash_file(&backup_destination);

            debug!("Original File Hash: {:?}", original_file_hash);
            debug!("New File hash: {:?}", new_file_hash);

            if hash_file(&destination)? == hash_file(&backup_destination)? {
                debug!("Removing backup file because it's the same as the original");
                remove_file(&backup_destination)?;
            } else {
                debug!("Keeping backup file because it differs from the original");
            }
        } else {
            // Encrypt the file and write the encrypted contents to the destination
            debug!(
                "Encrypting file at {:?} and storing contents at {:?}",
                source, destination
            );
            decrypt_file(&source, &destination, &encryption_key)?;
        }
    }

    Ok(())
}

pub fn write_encrypted_files_for_configuration(
    configuration: &Configuration,
    encryption_key: EncryptionKey,
) -> Result<(), ConfigureError> {
    let project_root = find_project_root()?;
    let secrets_root = find_secrets_repo()?;

    for file in &configuration.files_to_copy {
        let source = &secrets_root.join(&file.source);
        let destination = project_root.join(&file.get_encrypted_destination());

        create_parent_directory_for_path_if_not_exists(&destination)?;

        // Encrypt the file and write the encrypted contents to the destination
        debug!(
            "Encrypting file at {:?} and storing contents at {:?}",
            source, destination
        );

        encrypt_file(source, &destination, &encryption_key)?;
    }

    Ok(())
}

/// Returns the SHA-256 hash of a file at the given path
fn hash_file(path: &Path) -> Result<String, Error> {
    let input = File::open(path)?;
    let mut reader = BufReader::new(input);
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    let digest = context.finish();

    Ok(base64::encode(digest.as_ref()))
}

fn create_parent_directory_for_path_if_not_exists(path: &Path) -> Result<(), Error> {
    let parent = match path.parent() {
        Some(parent) => parent,
        None => return Ok(()), // if we're in the root of the filesystem, we have no work to do
    };

    create_dir_all(parent)
}

pub fn infer_encryption_output_filename(path: &Path) -> PathBuf {
    let mut string = path.to_path_buf().into_os_string().into_string().unwrap();
    string.push_str(".enc");
    Path::new(&string).to_path_buf()
}

pub fn infer_decryption_output_filename(path: &Path) -> PathBuf {
    let mut string = path.to_path_buf().into_os_string().into_string().unwrap();

    if !string.ends_with(".enc") {
        string.push_str(".decrypted");
        return Path::new(&string).to_path_buf();
    } else {
        let filename_without_suffix: String = string
            .chars()
            .take(string.chars().count() - ".enc".chars().count())
            .collect();
        return Path::new(&filename_without_suffix).to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    // Import the parent scope
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_find_project_root() {
        assert!(find_project_root().unwrap().exists());
    }

    #[test]
    fn test_get_configure_file_path_file_name_is_always_present() {
        assert!(get_configure_file_path().unwrap().file_name().is_some());
    }

    #[test]
    fn test_get_configure_file_path_contains_configure_file() {
        assert_eq!(
            get_configure_file_path().unwrap().file_name(),
            Some(OsStr::new(".configure"))
        );
    }

    #[test]
    fn test_find_configure_file_creates_it_if_missing() {
        delete_configure_file();
        find_configure_file().unwrap();
        assert!(get_configure_file_path().unwrap().exists());
    }

    #[test]
    fn test_encrypted_filename_can_be_derived_from_original_filename_for_files_with_extension() {
        let source = Path::new("/test.json").to_path_buf();
        let dest = Path::new("/test.json.enc").to_path_buf();

        assert_eq!(infer_encryption_output_filename(&source), dest)
    }

    #[test]
    fn test_encrypted_filename_can_be_derived_from_original_filename_for_files_without_extension() {
        let source = Path::new("/Gemfile").to_path_buf();
        let dest = Path::new("/Gemfile.enc").to_path_buf();

        assert_eq!(infer_encryption_output_filename(&source), dest)
    }

    #[test]
    fn test_decrypted_filename_can_be_derived_from_encrypted_filename_for_files_with_extension() {
        let source = Path::new("/test.json.enc").to_path_buf();
        let dest = Path::new("/test.json").to_path_buf();

        assert_eq!(infer_decryption_output_filename(&source), dest)
    }

    #[test]
    fn test_decrypted_filename_can_be_derived_from_original_filename_for_files_without_extension() {
        let source = Path::new("/Gemfile.enc").to_path_buf();
        let dest = Path::new("/Gemfile").to_path_buf();

        assert_eq!(infer_decryption_output_filename(&source), dest)
    }

    #[test]
    fn test_decrypted_filename_can_be_derived_from_original_filename_for_files_without_extension_or_suffix(
    ) {
        let source = Path::new("/Gemfile").to_path_buf();
        let dest = Path::new("/Gemfile.decrypted").to_path_buf();

        assert_eq!(infer_decryption_output_filename(&source), dest)
    }

    fn delete_configure_file() {
        if get_configure_file_path().unwrap().exists() {
            std::fs::remove_file(get_configure_file_path().unwrap()).unwrap();
        }
    }
}
