# a8c-secrets

A secure CLI tool for managing encrypted secrets in repositories from `~/.mobile-secrets`.

[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Overview

`a8c-secrets` provides a secure workflow for managing encrypted secret files in your repository. It uses AES-256-GCM encryption with unique keys per repository, ensuring that secrets are never stored in plain text in version control.

### Key Features

- 🔐 **AES-256-GCM Encryption**: Military-grade encryption for your secrets
- 🏠 **Centralized Storage**: All secrets stored in `~/.mobile-secrets`
- 🔑 **Repository-Specific Keys**: Each repository gets its own encryption key
- 🚀 **CI/CD Ready**: Environment variable support for automated deployments
- 🛡️ **Security First**: Path validation, git ignore checks, and secure defaults
- 📝 **Structured Logging**: Verbose mode for debugging and operational visibility

## Installation

### Prerequisites

- Rust 1.70 or later
- Git repository (for repository name detection)

### From Source

```bash
# Clone the repository
git clone https://github.com/automattic/a8c-secrets.git
cd a8c-secrets

# Build and install
cargo install --path .

# Verify installation
a8c-secrets --help
```

### From Cargo (when published)

```bash
cargo install a8c-secrets
```

## Quick Start

### 1. Initial Setup

```bash
# Navigate to your repository
cd /path/to/your/repo

# Run initial setup
a8c-secrets setup
```

This will:
- Create a `.a8c-secrets/config.yaml` configuration file
- Generate a unique AES-256 encryption key for your repository
- Store the key in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`
- Provide instructions for CI environment setup

### 2. Configure Secret Files

Edit `.a8c-secrets/config.yaml` to specify which secrets to sync:

```yaml
sha1: "abc123def456"
files:
  - source: "api-keys/production.env"
    destination: "config/secrets.env"
  - source: "certificates/ssl.pem"
    destination: "ssl/certificate.pem"
```

### 3. Encrypt Secrets

```bash
# Encrypt secrets from ~/.mobile-secrets into your repository
a8c-secrets encrypt
```

This creates encrypted `.a8c-secrets/*.enc` files that can be safely committed to version control.

### 4. Decrypt Secrets

```bash
# Decrypt secrets for local development or CI
a8c-secrets decrypt
```

This decrypts the secrets to their destination paths as specified in the config.

## Commands

### `setup`

Set up or validate secrets configuration for the current repository.

```bash
a8c-secrets setup
```

**What it does:**
- Creates `.a8c-secrets/config.yaml` if it doesn't exist
- Generates a unique AES-256 encryption key for this repository
- Stores the key in `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`
- Validates existing configuration if already set up

### `encrypt`

Encrypt secrets from `~/.mobile-secrets` into the current repository.

```bash
a8c-secrets encrypt
```

**What it does:**
- Updates the SHA1 in `.a8c-secrets/config.yaml` to match current `~/.mobile-secrets` git HEAD
- Reads each input secret file from `~/.mobile-secrets`
- Encrypts content using AES-256-GCM with the repository's unique key
- Saves encrypted data as `.a8c-secrets/*.enc` files

### `decrypt`

Decrypt secrets from `.a8c-secrets/*.enc` files to their destinations.

```bash
a8c-secrets decrypt
```

**What it does:**
- Reads `.a8c-secrets/config.yaml` configuration
- Decrypts each `.a8c-secrets/*.enc` file using the repository's encryption key
- Writes decrypted content to destination paths specified in the config
- Resolves relative destination paths relative to the repository root
- Creates missing destination directories as needed

**Key Sources:**
1. `A8C_SECRETS_ENCRYPTION_KEY` environment variable (preferred for CI)
2. `~/.mobile-secrets/a8c-secrets-encryption-keys.yaml` (for local development)

## Configuration

### Repository Configuration (`.a8c-secrets/config.yaml`)

```yaml
# SHA1 of the ~/.mobile-secrets repository HEAD (auto-updated by encrypt command)
sha1: "abc123def456"

# List of secret files to manage
files:
  # Simple file mapping
  - source: "api-keys/production.env"
    destination: "config/secrets.env"
  
  # Nested directory structure
  - source: "certificates/ssl.pem"
    destination: "ssl/certificate.pem"
  
  # External destination (outside repository)
  - source: "database/credentials.json"
    destination: "/etc/myapp/database.json"
```

### Encryption Keys File (`~/.mobile-secrets/a8c-secrets-encryption-keys.yaml`)

```yaml
# Repository names mapped to base64-encoded AES-256 keys
"automattic/wordpress-ios": "dGVzdC1rZXktZm9yLXdvcmRwcmVzcy1pb3MtMzI="
"automattic/wordpress-android": "YW5vdGhlci1rZXktZm9yLWFuZHJvaWQtMzI="
```

## Security Features

### Path Validation

- **Source paths**: Must be relative paths within `~/.mobile-secrets`
- **Destination paths**: Can be absolute or relative paths (relative paths are resolved relative to the repository root)
- **Traversal protection**: Prevents `../` attacks
- **Special character handling**: Validates UTF-8 and filesystem compatibility

### Git Integration

- **Repository detection**: Automatically detects current repository name
- **Remote validation**: Ensures `~/.mobile-secrets` is up-to-date
- **Detached HEAD handling**: Graceful handling of detached HEAD states
- **Git ignore checks**: Validates that decrypted files are ignored by git using repository-specific path resolution

### Encryption

- **AES-256-GCM**: Authenticated encryption with associated data
- **Unique nonces**: Each encryption uses a cryptographically secure random nonce
- **Key isolation**: Each repository has its own encryption key
- **Base64 encoding**: Keys stored in standard base64 format

## Environment Variables

### `A8C_SECRETS_ENCRYPTION_KEY`

Base64-encoded AES-256 encryption key for the current repository. Used primarily in CI/CD environments.

```bash
export A8C_SECRETS_ENCRYPTION_KEY="dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="
```

### `A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND`

Skip interactive prompts when `~/.mobile-secrets` is behind remote. Useful for automated scripts.

```bash
export A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND="yes"
```

## Examples

### Local Development Workflow

```bash
# 1. Set up the project (first time only)
a8c-secrets setup

# 2. Edit configuration to specify secrets
vim .a8c-secrets/config.yaml

# 3. Encrypt secrets from ~/.mobile-secrets
a8c-secrets encrypt

# 4. Decrypt for local development
a8c-secrets decrypt

# 5. Use the decrypted secrets in your application
```

### CI/CD Integration

```yaml
# Buildkite pipeline.yml example
steps:
  - label: "Build"
    commands: |
      echo "--- Decrypting secrets..."
      # Requirement: Ensure the a8c-secrets binary is installed in your agent
      # Also ensure A8C_SECRETS_ENCRYPTION_KEY env var is exposed to your pipeline secrets
      a8c-secrets decrypt

      echo "--- Building..."
      # Decrypted secrets are now available, you can run your build steps
```

### Multiple Environments

```yaml
# .a8c-secrets/config.yaml
sha1: "abc123def456"
files:
  # Development secrets
  - source: "env/development.env"
    destination: "config/development.env"
  
  # Production secrets
  - source: "env/production.env"
    destination: "config/production.env"
  
  # Staging secrets
  - source: "env/staging.env"
    destination: "config/staging.env"
```

## Troubleshooting

### Common Issues

**"No encryption key available for this repository"**
```bash
# Run setup to generate a key
a8c-secrets setup
```

**"Invalid base64 encoding"**
```bash
# Ensure the environment variable contains valid base64
echo $A8C_SECRETS_ENCRYPTION_KEY | base64 -d > /dev/null
```

**"~/.mobile-secrets repository is behind origin/trunk"**
```bash
# Update the mobile-secrets repository
cd ~/.mobile-secrets && git checkout trunk && git pull
```

**"Destination file is NOT ignored by git"**
```bash
# Add the destination path to .gitignore
echo "config/secrets.env" >> .gitignore
```

### Verbose Logging

Enable detailed logging for debugging:

```bash
a8c-secrets --verbose encrypt
a8c-secrets --verbose decrypt
```

## Development

### Building from Source

```bash
git clone https://github.com/automattic/a8c-secrets.git
cd a8c-secrets
cargo build
```

**Note**: This project includes a `Makefile` with convenient targets for common development tasks:
- `make test` - Run all tests sequentially
- `make lint` - Run clippy linter
- `make fmt` - Format code
- `make build` - Build the project

### Running Tests

```bash
# Run all tests (sequentially to avoid race conditions)
make test

# Or run with cargo directly
cargo test -- --test-threads=1

# Run specific test modules
cargo test --lib crypto::tests
cargo test --lib git::tests
cargo test --lib paths::tests

# Run integration tests
cargo test --test integration_tests
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check
```

### Documentation

```bash
# Generate and open API documentation
cargo doc --open

# Generate documentation for private items
cargo doc --document-private-items --open
```

## Architecture

### Module Structure

- **`main.rs`**: CLI entry point and argument parsing
- **`lib.rs`**: Public API and shared types
- **`commands.rs`**: Command implementations (setup, encrypt, decrypt)
- **`crypto.rs`**: Encryption/decryption operations
- **`git.rs`**: Git repository operations and validation
- **`paths.rs`**: Path resolution and configuration loading

### Key Design Principles

1. **Security First**: All security decisions prioritize safety over convenience
2. **Explicit Configuration**: No magic defaults, everything must be explicitly configured
3. **Error Handling**: Comprehensive error messages with actionable guidance
4. **Testability**: Extensive test coverage with realistic scenarios
5. **CI/CD Ready**: Environment variable support for automated deployments

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust coding standards and Clippy recommendations
- Add tests for new functionality
- Update documentation for API changes
- Ensure all tests pass before submitting PR

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For questions, issues, or contributions:

- **Issues**: [GitHub Issues](https://github.com/automattic/a8c-secrets/issues)
- **Discussions**: [GitHub Discussions](https://github.com/automattic/a8c-secrets/discussions)
- **Security**: [Security Policy](https://github.com/automattic/a8c-secrets/security/policy)

---

**Note**: This tool is designed for internal use at Automattic. Please ensure you have proper authorization before using it in your environment.
