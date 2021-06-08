use crate::fs::*;
use crate::git::*;
use crate::ui::*;
use chrono::prelude::*;
use indicatif::ProgressBar;

use console::style;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub project_name: String,
    pub branch: String,
    pub pinned_hash: String,
    pub files_to_copy: Vec<File>,
}

impl Configuration {
    pub fn is_empty(&self) -> bool {
        self == &Configuration::default()
    }

    pub fn from_str(string: String) -> Result<Configuration, ConfigureError> {
        match serde_json::from_str(&string) {
            Ok(configuration) => Ok(configuration),
            Err(_) => Err(ConfigureError::ConfigureFileNotValid),
        }
    }

    pub fn to_string(&self) -> Result<String, ConfigureError> {
        match serde_json::to_string_pretty(&self) {
            Ok(string) => Ok(string),
            Err(_) => Err(ConfigureError::ConfigureDataNotValid),
        }
    }

    pub fn set_pinned_hash_from_repo(&mut self, repo: &SecretsRepo) {
        let latest_hash = repo
            .latest_local_hash_for_branch(&self.branch)
            .expect("Unable to fetch the latest secrets hash");

        self.pinned_hash = latest_hash;
    }

    fn needs_project_name(&self) -> bool {
        self.project_name.is_empty()
    }

    fn needs_branch(&self) -> bool {
        self.branch.is_empty()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        let files_to_copy: Vec<File> = Vec::new();
        Configuration {
            project_name: "".to_string(),
            branch: "".to_string(),
            pinned_hash: "".to_string(),
            files_to_copy,
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigureError {
    #[error("Unable to decrypt file")]
    DataDecryptionError(#[from] std::io::Error),

    #[error("Unknown git error")]
    SecretsRepoError(#[from] git2::Error),

    #[error("Invalid git status")]
    GitStatusParsingError(#[from] std::num::ParseIntError),

    #[error("Unable to find current secrets repo branch")]
    GitGetCurrentBranchError,

    #[error("Invalid git status")]
    GitStatusUnknownError,

    #[error("Unable to find the root of the respository – are you sure you're running this inside a git repo?")]
    ProjectNotPresent,

    #[error("The .configure file is missing or could not be read")]
    ConfigureFileNotReadable,

    #[error("The .configure file could not be written")]
    ConfigureFileNotWritable,

    #[error("Unable to parse configuration file – the JSON is probably invalid")]
    ConfigureFileNotValid,

    #[error("Unable to save an configuration data – it couldn't be converted to JSON")]
    ConfigureDataNotValid,

    #[error("No secrets repository could be found on this machine")]
    SecretsNotPresent,

    #[error("An encrypted file is missing – unable to apply secrets to project. Run `configure update` to fix this")]
    EncryptedFileMissing,

    #[error("Unable to read keys.json file in your secrets repo")]
    KeysFileNotReadable,

    #[error("Unable to write keys.json file in your secrets repo")]
    KeysFileNotWritable,

    #[error("keys.json file in your secrets repo is not valid – it might be invalid JSON, or it could be structured incorrectly")]
    KeysFileIsNotValid,

    #[error("Attempted to save invalid keys.json data – it couldn't be converted to JSON")]
    KeysDataIsNotValid,

    #[error("That project key is not defined in keys.json")]
    MissingProjectKey,

    #[error("This decryption key is not valid base64")]
    DecryptionKeyEncodingError,

    #[error("This decryption key is not a sodium-compatible key")]
    DecryptionKeyParsingError,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct File {
    #[serde(rename = "file")]
    pub source: String,
    pub destination: String,
}

impl File {
    pub fn get_encrypted_destination(&self) -> String {
        // This monstrosity tries to ensure we put files in the `.configure-files` directory for temporary storage. If something goes wrong,
        // we fall back to just putting the file where it's specified to go
        if let Ok(project_root) = find_project_root() {
            let destination = Path::new(&self.destination);
            if let Some(os_file_name) = destination.file_name() {
                if let Some(file_name) = os_file_name.to_str() {
                    if let Some(destination) = project_root
                        .join(".configure-files")
                        .join(file_name.to_owned() + &".enc".to_owned())
                        .to_str()
                    {
                        return destination.to_string();
                    }
                }
            }
        }

        self.destination.clone() + &".enc".to_owned()
    }

    pub fn get_decrypted_destination(&self) -> String {
        self.destination.clone()
    }

    pub fn get_backup_destination(&self) -> PathBuf {
        self.get_backup_destination_for_date(Utc::now())
    }

    fn get_backup_destination_for_date(&self, date: DateTime<Utc>) -> PathBuf {
        let path = Path::new(&self.destination);

        let directory = path.parent().unwrap_or_else(|| Path::new("/")); // If we're at the root of the file system

        let file_stem = path
            .file_stem()
            .unwrap_or_default() // Ensure one exists
            .to_str() // Convert from OsStr
            .unwrap_or_default(); // Blank on failure

        let extension = path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let datetime = date.format("%Y-%m-%d-%H-%M-%S").to_string();

        let filename: String;

        if extension.is_empty() {
            filename = format!("{:}-{:}.bak", file_stem, datetime);
        } else {
            filename = format!("{:}-{:}.{:}.bak", file_stem, datetime, extension);
        }

        directory.join(filename)
    }
}

pub fn apply_configuration(configuration: &Configuration) {
    // Decrypt the project's configuration files
    decrypt_files_for_configuration(&configuration).expect("Unable to decrypt and copy files");

    debug!("All Files Copied!");

    info!("Done")
}

pub fn update_configuration(
    configuration_file_path: Option<String>,
    interactive: bool,
) -> Configuration {
    let mut configuration = read_configuration_from_file(&configuration_file_path)
        .expect("Unable to read configuration from `.configure` file");

    let secrets_repo = SecretsRepo::default();
    let starting_branch = secrets_repo
        .current_branch()
        .expect("Unable to determine current mobile secrets branch");
    let starting_ref = secrets_repo
        .current_hash()
        .expect("Unable to determine current mobile secrets commit hash");

    heading("Configure Update");

    //
    // Step 1 – Fetch the latest mobile secrets from the server
    //          We need them in order to update the pinned hash
    //
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(125);
    bar.set_message("Fetching Latest Mobile Secrets");

    secrets_repo
        .update_local_copy()
        .expect("Unable to fetch latest mobile secrets");

    bar.finish_and_clear();

    //
    // Step 2 – Check if the user wants to use a different secrets branch
    //
    if interactive {
        configuration = prompt_for_branch(&secrets_repo, configuration, true);
    }

    //
    // Step 3 – Check if the current configuration branch is in sync with the server or not.or
    // If not, check with the user whether they'd like to continue
    //
    let status = secrets_repo
        .status()
        .expect("Unable to get secrets repo status");

    debug!("Repo status is: {:?}", status);

    let should_continue = !interactive
        || match status.sync_state {
            RepoSyncState::Ahead => {
                warn(&format!(
                    "Your local secrets repo has {:?} change(s) that the server does not",
                    status.distance
                ));

                confirm("Would you like to continue?")
            }
            RepoSyncState::Behind => {
                warn(&format!(
                    "The server has {:?} change(s) that your local secrets repo does not",
                    status.distance
                ));

                confirm("Would you like to continue?")
            }
            RepoSyncState::Synced => true,
        };

    if !should_continue {
        debug!("Exiting without updating hash");
        return configuration;
    }

    //
    // Step 4 – Check if the project's secrets are out of date compared to the server.
    //          If they out of date, we'll prompt the user to pull the latest remote
    //          changes into the local secrets repo before continuing.
    //
    let distance = secrets_repo.commits_ahead_of_configuration(&configuration);
    debug!(
        "The project is {:} commit(s) behind the latest secrets",
        distance
    );

    // Update the pinned hash when nothing has changed – this helps fill in the blanks when creating a `.configure` file by hand
    if distance == 0 {
        let latest_commit_hash = secrets_repo
            .latest_remote_hash_for_branch(&configuration.branch)
            .expect("Unable to fetch latest commit hash");
        configuration.pinned_hash = latest_commit_hash;
    } else {
        let message = format!(
            "This project is {:} commit(s) behind the latest secrets. Would you like to use the latest secrets?",
            distance
        );

        // Prompt to update to most recent secrets data in the branch (if we're in interactive mode – if not, just do it)
        if !interactive || confirm(&message) {
            let latest_commit_hash = secrets_repo
                .latest_remote_hash_for_branch(&configuration.branch)
                .expect("Unable to fetch latest commit hash");

            debug!(
                "Moving the secrets repo to {:?} at {:?}",
                &configuration.branch, latest_commit_hash
            );

            secrets_repo
                .switch_to_branch_at_revision(&configuration.branch, &latest_commit_hash)
                .expect("Unable to check out branch at revision");

            // Update the pinned hash in `.configure` file before continuing
            debug!(
                "Updating the .configure file pinned hash to {:?}",
                latest_commit_hash
            );
            configuration.pinned_hash = latest_commit_hash;
        }
    }

    //
    // Step 5 – Write out encrypted files as needed
    //
    let configure_file_path = resolve_configure_file_path(&configuration_file_path).expect("");
    write_configuration_to(&configuration, &configure_file_path)
        .expect("Unable to write configuration");

    //
    // Step 6 – Write out encrypted files as needed
    //
    let encryption_key =
        encryption_key_for_configuration(&configuration).expect("Unable to find encryption key");
    write_encrypted_files_for_configuration(&configuration, encryption_key)
        .expect("Unable to copy encrypted files");

    //
    // Step 7 – Roll the secrets repo back to how it was before we started
    //
    secrets_repo
        .switch_to_branch_at_revision(&starting_branch, &starting_ref)
        .expect("Unable to roll back to branch");

    //
    // Step 8 – Apply these changes to the current repo
    //
    apply_configuration(&configuration);

    //
    // Step 9 - All done!
    //
    configuration
}

pub fn validate_configuration(configuration: Configuration) {
    println!("{:?}", configuration);
}

pub fn setup_configuration(mut configuration: Configuration) {
    heading("Configure Setup");
    println!("Let's get configuration set up for this project.");
    newline();

    let repo = SecretsRepo::default();

    // Help the user set the `project_name` field
    configuration = prompt_for_project_name_if_needed(configuration);

    // Help the user set the `branch` field
    configuration = prompt_for_branch(&repo, configuration, true);

    // Set the latest automatically hash based on the selected branch
    configuration.set_pinned_hash_from_repo(&repo);

    // Help the user add files
    configuration = prompt_to_add_files(configuration);

    info!("Writing changes to .configure");

    write_configuration(&configuration).expect("Unable to save configure file");

    // Create a key in `keys.json` for the project if one doesn't already exist
    generate_encryption_key_if_needed(&configuration)
        .expect("Unable to generate an encryption key for this project");
}

fn prompt_for_project_name_if_needed(mut configuration: Configuration) -> Configuration {
    // If there's already a project name, don't bother updating it
    if !configuration.needs_project_name() {
        return configuration;
    }

    let project_name = prompt("What is the name of your project?");
    configuration.project_name = project_name.clone();
    println!("Project Name set to: {:?}", project_name);

    configuration
}

fn prompt_for_branch(
    repo: &SecretsRepo,
    mut configuration: Configuration,
    force: bool,
) -> Configuration {
    // If there's already a branch set, don't bother updating it
    if !configuration.needs_branch() && !force {
        return configuration;
    }

    let current_branch = repo
        .current_branch()
        .expect("Unable to determine current mobile secrets branch");
    let branches = repo
        .local_branch_names()
        .expect("Unable to fetch mobile secrets branches");

    println!("Using the secrets repository at {:?}", repo.path);
    newline();
    println!("Which branch would you like to use?");
    println!("Current Branch: {}", style(&current_branch).green());

    let selected_branch =
        select(branches, &current_branch).expect("Unable to read selected branch");

    configuration.branch = selected_branch.clone();
    println!("Secrets repo branch set to: {:?}", selected_branch);

    configuration
}

fn prompt_to_add_files(mut configuration: Configuration) -> Configuration {
    let mut files = configuration.files_to_copy;

    let mut message = "Would you like to add files?";

    if !files.is_empty() {
        message = "Would you like to add additional files?";
    }

    while confirm(message) {
        match prompt_to_add_file() {
            Some(file) => files.push(file),
            None => continue,
        }
    }

    configuration.files_to_copy = files;

    configuration
}

fn prompt_to_add_file() -> Option<File> {
    let relative_source_file_path =
        prompt("Enter the source file path (relative to the secrets root):");

    let secrets_root = match find_secrets_repo() {
        Ok(repo_path) => repo_path,
        Err(_) => return None,
    };

    let full_source_file_path = secrets_root.join(&relative_source_file_path);

    if !full_source_file_path.exists() {
        println!("Source File does not exist: {:?}", full_source_file_path);
        return None;
    }

    let relative_destination_file_path =
        prompt("Enter the destination file path (relative to the project root):");

    let project_root = find_project_root().unwrap();
    let full_destination_file_path = project_root.join(&relative_destination_file_path);

    debug!("Destination: {:?}", full_destination_file_path);

    Some(File {
        source: relative_source_file_path,
        destination: relative_destination_file_path,
    })
}

#[cfg(test)]
mod tests {
    // Import the parent scope
    use super::*;

    #[test]
    fn test_that_default_configuration_needs_project_name() {
        assert!(Configuration::default().needs_project_name())
    }

    #[test]
    fn test_that_default_configuration_needs_branch() {
        assert!(Configuration::default().needs_branch())
    }

    #[test]
    fn test_that_default_configuration_pinned_hash_is_empty() {
        assert!(Configuration::default().pinned_hash == "")
    }

    #[test]
    fn test_that_invalid_configuration_cannot_be_deseralized() {
        assert!(Configuration::from_str("".to_string()).is_err())
    }

    #[test]
    fn test_that_default_configuration_can_be_serialized() {
        assert!(Configuration::default().to_string().is_ok())
    }

    #[test]
    fn test_that_default_configuration_is_empty() {
        assert!(Configuration::default().is_empty())
    }

    #[test]
    fn test_that_get_encrypted_destination_ends_in_enc_extension() {
        let file = File {
            source: "".to_string(),
            destination: ".configure-files/file".to_string(),
        };
        assert_eq!(
            Path::new(&file.get_encrypted_destination())
                .file_name()
                .unwrap(),
            "file.enc"
        )
    }

    #[test]
    fn test_that_get_encrypted_destination_with_extension_ends_in_enc_extension() {
        let file = File {
            source: "".to_string(),
            destination: ".configure-files/file.txt".to_string(),
        };
        assert_eq!(
            Path::new(&file.get_encrypted_destination())
                .file_name()
                .unwrap(),
            "file.txt.enc"
        )
    }

    #[test]
    fn test_that_get_encrypted_destination_with_final_destination_outside_configurefiles_directory_is_correct(
    ) {
        let file = File {
            source: "".to_string(),
            destination: "foo/bar/file".to_string(),
        };
        assert_eq!(
            Path::new(&file.get_encrypted_destination())
                .parent()
                .unwrap()
                .file_name()
                .unwrap(),
            ".configure-files"
        )
    }

    #[test]
    fn test_that_get_decrypted_destination_matches_starting_filename() {
        let file = File {
            source: "".to_string(),
            destination: ".configure-files/file".to_string(),
        };
        assert_eq!(file.get_decrypted_destination(), ".configure-files/file")
    }

    #[test]
    fn test_that_get_backup_destination_has_bak_extension() {
        let file = File {
            source: "".to_string(),
            destination: ".configure-files/file".to_string(),
        };
        assert_eq!(file.get_backup_destination().extension().unwrap(), "bak")
    }

    #[test]
    fn test_that_get_backup_destination_works_for_files_in_filesystem_root() {
        let file = File {
            source: "".to_string(),
            destination: "/.configure-files/file.txt".to_string(),
        };
        assert_eq!(
            file.get_backup_destination_for_date(get_zero_date()),
            Path::new("/.configure-files").join("file-1970-01-01-00-00-00.txt.bak")
        )
    }

    #[test]
    fn test_that_get_backup_destination_works_for_files_without_extension() {
        let file = File {
            source: "".to_string(),
            destination: ".configure-files/file".to_string(),
        };
        assert_eq!(
            file.get_backup_destination_for_date(get_zero_date()),
            Path::new(".configure-files").join("file-1970-01-01-00-00-00.bak")
        )
    }

    fn get_zero_date() -> DateTime<Utc> {
        Utc.timestamp(0, 0)
    }
    // #[test]
    // fn test_that_pinned_hash_is_updated_when_running_update_on_empty_file() {
    //     use_test_keys();
    //     let mut test_conf = ConfigurationFile::default();
    //     test_conf.project_name = "Test Project 1".to_string();
    //     test_conf.branch = "trunk".to_string();

    //     update_configuration(test_conf, false);
    // }

    // fn use_test_keys() {
    //     let project_root = std::env::current_dir().unwrap();
    //     let tests_dir = project_root.join("tests");
    //     assert!(tests_dir.exists() && tests_dir.is_dir());
    //     std::env::set_var(SECRETS_KEY_NAME, tests_dir);
    // }
}
