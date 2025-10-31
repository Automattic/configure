use anyhow::Result;
use clap::{Parser, Subcommand};

use a8c_secrets::commands::{decrypt_command, encrypt_command, setup_command};

#[derive(Parser)]
#[command(name = "a8c-secrets")]
#[command(
    about = "A CLI tool for managing encrypted secrets in repositories from ~/.mobile-secrets"
)]
#[command(long_about = r#"
a8c-secrets - Automattic Secrets Management Tool

This tool manages encrypted secret files in your repository. It provides a secure workflow for:

• Setting up or validating the initial a8c-secrets configuration in your repository (`setup`)
• Encrypting new secrets, or updating existing ones, from `~/.mobile-secrets` into your repository (`encrypt`)
• Decrypting secrets from `.a8c-secrets/*.enc` files for you to use in local development or CI (`decrypt`)

The tool uses AES-256-GCM encryption with unique keys per repository, stored in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`.
Each project maintains a `.a8c-secrets/config.yaml` configuration file that tracks which secrets to sync and where to place them when decrypted.
"#)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

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
• Provides instructions for setting up the key in CI environments

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
• Updates the SHA1 in `.a8c-secrets/config.yaml` to match current `~/.mobile-secrets` git HEAD SHA1
• Reads each input secret file (specified in the configuration) from `~/.mobile-secrets`
• Encrypts their content using AES-256-GCM, using the repository's unique key
• Saves the encrypted data as `.a8c-secrets/*.enc` files in the current repository

Run this command whenever you want to encrypt new secrets or update the encrypted secrets in your
repository with the latest versions from `~/.mobile-secrets`.
"#
    )]
    Encrypt,

    #[command(about = "Decrypt secrets from `.a8c-secrets/*.enc` files to their destinations")]
    #[command(
        long_about = r#"Decrypt secrets from `.a8c-secrets/*.enc` files to their destinations.

This command:
• Reads the `.a8c-secrets/config.yaml` configuration file
• Decrypts each `.a8c-secrets/*.enc` file using the repository's encryption key
• Writes the decrypted content to the destination paths specified in the config

It creates any missing destination directories as needed.

The decryption key is obtained from:
1. The `A8C_SECRETS_ENCRYPTION_KEY` environment variable (preferred for CI) if it exists
2. Or reading it from `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml` (for local development when the environment variable is not set)

Use this command in local development or CI to make the secret files decrypted and available in the right places before compilation.
"#
    )]
    Decrypt,
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

    // Initialize logging
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();

    tracing::info!("Starting a8c-secrets");

    let result = match cli.command {
        Commands::Setup => setup_command(&std::env::current_dir()?),
        Commands::Encrypt => encrypt_command(&std::env::current_dir()?),
        Commands::Decrypt => decrypt_command(&std::env::current_dir()?),
    };

    match &result {
        Ok(()) => tracing::info!("Command completed successfully"),
        Err(e) => tracing::error!("Command failed: {}", e),
    }

    result
}
