use crate::encryption::{decrypt_file, encrypt_file};
use crate::ConfigurationFile;
use crate::ConfigureError;
use log::{debug, info};
use ring::digest::{Context, SHA256};
use std::env;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{BufReader, Error, Read, Write};
use std::path::PathBuf;

/// Find the .configure file in the current project
pub fn find_configure_file() -> PathBuf {
    let project_root = find_project_root();

    let configure_file_path = project_root.join(".configure");

    debug!("Configure file found at: {:?}", configure_file_path);

    if !configure_file_path.exists() {
        info!(
            "No configure file found at: {:?}. Creating one for you",
            configure_file_path
        );

        save_configuration(&ConfigurationFile::default())
            .expect("There is no `configure.json` file in your project, and creating one failed");
    }

    configure_file_path
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
        create_file_with_contents(&keys_file_path, "{}").expect(
            "There is no `keys.json` file in your secrets repository, and creating one failed",
        );
    }

    Ok(keys_file_path)
}

pub fn find_project_root() -> PathBuf {
    let path = env::current_dir().expect("Unable to determine current directory");

    let repo = git2::Repository::discover(&path)
        .expect("Unable to find the root of the respository – are you sure you're running this inside a git repo?");

    debug!("Discovered Repository at {:?}", &path);

    repo.workdir().unwrap().to_path_buf()
}

pub fn find_secrets_repo() -> Result<PathBuf, ConfigureError> {
    // TODO: Allow the user to set their own mobile secrets path using an environment variable

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

pub fn read_configuration() -> ConfigurationFile {
    let configure_file_path = find_configure_file();
    let mut file = File::open(&configure_file_path).expect("Unable to open configuration file");

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .expect("Unable to read configuration file");

    let result: ConfigurationFile = serde_json::from_str(&file_contents)
        .expect("Unable to parse configuration file – the JSON is probably invalid");

    result
}

pub fn save_configuration(configuration: &ConfigurationFile) -> Result<(), Error> {
    let serialized = serde_json::to_string_pretty(&configuration)?;

    let configure_file = find_configure_file();

    debug!("Writing to: {:?}", configure_file);

    let mut file = File::create(configure_file)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

pub fn decrypt_files_for_configuration(
    configuration: &ConfigurationFile,
) -> Result<(), ConfigureError> {
    let project_root = find_project_root();
    let encryption_key = configuration.get_encryption_key();

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
) -> Result<(), Error> {
    let project_root = find_project_root();
    let secrets_root = find_secrets_repo().unwrap();
    let encryption_key = configuration.get_encryption_key();

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
fn create_file_with_contents(path: &PathBuf, contents: &str) -> Result<(), std::io::Error> {
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
