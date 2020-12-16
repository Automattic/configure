use crate::encryption::{decrypt_file, encrypt_file, generate_key};
use crate::ConfigurationFile;
use crate::ConfigureError;
use log::{debug, info};
use ring::digest::{Context, SHA256};

use std::collections::HashMap;
use std::env;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{BufReader, Error, Read, Write};
use std::path::Path;
use std::path::PathBuf;

/// Find the .configure file in the current project
pub fn find_configure_file() -> PathBuf {
    let configure_file_path = get_configure_file_path().unwrap();

    if !configure_file_path.exists() {
        info!(
            "No configure file found at: {:?}. Creating one for you",
            configure_file_path
        );

        save_configuration_to(&ConfigurationFile::default(), &configure_file_path)
            .expect("There is no `configure.json` file in your project, and creating one failed");
    }

    debug!("Configure file found at: {:?}", configure_file_path);

    configure_file_path
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
        write_file_with_contents(&keys_file_path, "{}").expect(
            "There is no `keys.json` file in your secrets repository, and creating one failed",
        );
    }

    Ok(keys_file_path)
}

pub fn find_project_root() -> Result<PathBuf, ConfigureError> {
    let path = env::current_dir().expect("Unable to determine current directory");

    let repo = match git2::Repository::discover(&path) {
        Ok(repo) => repo,
        Err(_) => return Err(ConfigureError::ProjectNotPresent)
    };

    debug!("Discovered Repository at {:?}", &path);

    let project_root = match repo.workdir() {
        Some(dir) => dir,
        None => return Err(ConfigureError::ProjectNotPresent)
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

pub fn read_configuration() -> Result<ConfigurationFile, ConfigureError> {
    let configure_file_path = find_configure_file();

    let mut file = match File::open(&configure_file_path) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::ConfigureFileNotReadable),
    };

    let mut file_contents = String::new();
    match file.read_to_string(&mut file_contents) {
        Ok(_) => assert!(true), // no-op
        Err(_) => return Err(ConfigureError::ConfigureFileNotReadable),
    };

    match serde_json::from_str(&file_contents) {
        Ok(configuration) => return Ok(configuration),
        Err(_) => return Err(ConfigureError::ConfigureFileNotValid),
    }
}

pub fn save_configuration(configuration: &ConfigurationFile) -> Result<(), Error> {
    save_configuration_to(configuration, &find_configure_file())
}

fn save_configuration_to(
    configuration: &ConfigurationFile,
    configure_file: &PathBuf,
) -> Result<(), Error> {
    let serialized = serde_json::to_string_pretty(&configuration)?;

    debug!("Writing to: {:?}", configure_file);

    let mut file = File::create(configure_file)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

pub fn generate_encryption_key_if_needed(
    configuration: &ConfigurationFile,
) -> Result<(), ConfigureError> {
    if encryption_key_for_configuration(&configuration).is_ok() {
        return Ok(());
    }

    let keys_file_path = find_keys_file()?;

    let mut keys = read_keys(&keys_file_path)?;
    keys.insert(configuration.project_name.to_string(), generate_key());

    save_keys(&keys_file_path, &keys)
}

pub fn encryption_key_for_configuration(
    configuration: &ConfigurationFile,
) -> Result<String, ConfigureError> {
    let keys_file_path = find_keys_file()?;

    debug!("Reading keys from {:?}", keys_file_path);

    let keys = read_keys(&keys_file_path)?;

    match keys.get(&configuration.project_name) {
        Some(key) => Ok(key.to_string()),
        None => return Err(ConfigureError::MissingProjectKey),
    }
}

fn read_keys(source: &PathBuf) -> Result<HashMap<String, String>, ConfigureError> {
    let file = match File::open(&source) {
        Ok(file) => file,
        Err(_) => return Err(ConfigureError::KeysFileCannotBeRead),
    };

    let map: HashMap<String, String> = match serde_json::from_reader(file) {
        Ok(map) => map,
        Err(_) => return Err(ConfigureError::KeysFileIsNotValid),
    };

    Ok(map)
}

fn save_keys(destination: &PathBuf, keys: &HashMap<String, String>) -> Result<(), ConfigureError> {
    write_file_with_contents(destination, &serde_json::to_string_pretty(&keys).unwrap())?;
    Ok(())
}

pub fn decrypt_files_for_configuration(
    configuration: &ConfigurationFile,
) -> Result<(), ConfigureError> {
    let project_root = find_project_root()?;

    let encryption_key;

    // Allow defining an environment variable that can override the key selection (for use in CI, for example).
    // This is placed here instead of in `read_encryption_key` because it isa security risk to allow this override for
    // encryption – someone might set the encryption key on their local machine, causing every project to silently use the same key.
    if let Ok(var) = env::var(crate::ENCRYPTION_KEY_NAME) {
        encryption_key = var;
    } else {
        encryption_key = encryption_key_for_configuration(configuration)?;
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
    configuration: &ConfigurationFile,
    encryption_key: String,
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

        encrypt_file(&source, &destination, &encryption_key)?;
    }

    Ok(())
}

/// Helper method to create an empty file
fn write_file_with_contents(path: &PathBuf, contents: &str) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

/// Returns the SHA-256 hash of a file at the given path
fn hash_file(path: &PathBuf) -> Result<String, Error> {
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

fn create_parent_directory_for_path_if_not_exists(path: &PathBuf) -> Result<(), Error> {
    let parent = match path.parent() {
        Some(parent) => parent,
        None => return Ok(()), // if we're in the root of the filesystem, we have no work to do
    };

    Ok(create_dir_all(parent)?)
}

#[cfg(test)]
mod tests {
    // Import the parent scope
    use super::*;

    #[test]
    fn test_find_configure_file_creates_it_if_missing() {
        delete_configure_file();
        find_configure_file();
        assert!(get_configure_file_path().exists());
    }

    fn delete_configure_file() {
        if get_configure_file_path().exists() {
            std::fs::remove_file(get_configure_file_path()).unwrap();
        }
    }
}
