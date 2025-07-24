use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

mod commands;
mod utils;

use commands::{setup_command, update_command, apply_command};

// Constants for file names
pub const REPO_SECRETS_CONFIG_FILE: &str = "config.yaml";
pub const MOBILE_SECRETS_ENCRYPTION_KEYS_FILE: &str = "a8c-secrets-encryption-keys.yaml";
pub const REPO_SECRETS_DIR: &str = ".a8c-secrets";
pub const ENV_VAR_KEY: &str = "A8C_SECRETS_ENCRYPTION_KEY";

#[derive(Parser)]
#[command(name = "a8c-secrets")]
#[command(
    about = "A CLI tool for managing encrypted secrets in repositories from ~/.mobile-secrets"
)]
#[command(long_about = r#"
a8c-secrets - Automattic Secrets Management Tool

This tool manages encrypted secret files in your repository. It provides a secure workflow for:

• Setting up or validating the initial a8c-secrets configuration in your repository (`setup`)
• Encrypting new secrets, or updating existing ones, from `~/.mobile-secrets` into your repository (`update`)
• Decrypting secrets from `.a8c-secrets/*.enc` files for you to use in local development or CI (`apply`)

The tool uses AES-256-GCM encryption with unique keys per repository, stored in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`.
Each project maintains a `.a8c-secrets/config.yaml` configuration file that tracks which secrets to sync and where to place them when decrypted.
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Set up or validate secrets configuration for the current repository")]
    #[command(
        long_about = r#"Set up or validate secrets configuration for the current repository.

This command will:
• If no setup exists: Create a `.a8c-secrets/config.yaml` configuration file and generate encryption key
• If setup exists: Validate and display the current configuration

For initial setup:
• Creates a `.a8c-secrets/config.yaml` configuration file in the current directory
• Generates a unique AES-256 encryption key for this repository
• Stores the key in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`
• Provides instructions for setting up the key in CI/CD environments

For existing setup:
• Validates the `.a8c-secrets/config.yaml` configuration file structure
• Checks if encryption key exists for this repository
• Displays current configuration status

After initial setup, you'll need to manually edit .a8c-secrets/config.yaml to
specify which secret files to sync from ~/.mobile-secrets.
"#
    )]
    Setup,

    #[command(about = "Encrypt secrets from `~/.mobile-secrets` into the current repository")]
    #[command(
        long_about = r#"Encrypt secrets from `~/.mobile-secrets` into the current repository.

This command:
• Updates the SHA1 in `.a8c-secrets/config.yaml` to match current `~/.mobile-secrets` HEAD
• Reads each secret file specified in the configuration from `~/.mobile-secrets`
• Encrypts the content using AES-256-GCM with the repository's unique key
• Saves encrypted files as `.a8c-secrets/*.enc` in the current repository

Run this command whenever you want to encrypt new secrets or update the encrypted secrets in your
repository with the latest versions from `~/.mobile-secrets`.
"#
    )]
    Update,

    #[command(about = "Decrypt secrets from `.a8c-secrets/*.enc` files to their destinations")]
    #[command(
        long_about = r#"Decrypt secrets from `.a8c-secrets/*.enc` files to their destinations.

This command:
• Reads the `.a8c-secrets/config.yaml` configuration
• Decrypts each `.a8c-secrets/*.enc` file using the repository's encryption key
• Writes decrypted content to the destination paths specified in the config
• Creates destination directories as needed

The decryption key is obtained from:
1. A8C_SECRETS_ENCRYPTION_KEY environment variable (preferred for CI)
2. Or `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml` (for local development when the environment variable is not set)

Use this command in local development or CI to make the secret files decrypted and available in the right places before compilation.
"#
    )]
    Apply,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub sha1: String,
    pub files: Vec<SecretFile>,
}

#[derive(Serialize, Deserialize)]
pub struct SecretFile {
    pub source: String,
    pub destination: String,
}

/// Main entry point for the a8c-secrets CLI tool.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand handler.
///
/// # Returns
/// - `Ok(())` if the command executes successfully
/// - `Err(anyhow::Error)` if any error occurs during execution
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => setup_command(),
        Commands::Update => update_command(),
        Commands::Apply => apply_command(),
    }
}