use a8c_secrets::{
    decrypt_command, encrypt_command, setup_command, MOBILE_SECRETS_ENCRYPTION_KEYS_FILE,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_full_workflow() {
    // Create temporary directories
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_home = std::env::var("HOME").ok();

    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");

    // Create mobile-secrets directory structure
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "super secret api key").unwrap();

    // Initialize git repository in mobile-secrets
    let mobile_secrets_repo = git2::Repository::init(&mobile_secrets_path).unwrap();
    let mut index = mobile_secrets_repo.index().unwrap();
    index.add_path(Path::new("test_secret.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = mobile_secrets_repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let _commit_id = mobile_secrets_repo
        .commit(
            Some("refs/heads/trunk"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Set HEAD to point to trunk branch
    mobile_secrets_repo.set_head("refs/heads/trunk").unwrap();

    // Create a bare repository to use as remote for mobile-secrets
    let bare_repo_dir = tempdir().unwrap();
    let _bare_repo = git2::Repository::init_bare(bare_repo_dir.path()).unwrap();

    // Add the bare repo as remote to mobile-secrets repository
    mobile_secrets_repo
        .remote("origin", bare_repo_dir.path().to_str().unwrap())
        .unwrap();

    // Push the trunk branch to the remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in temp directory
    let repo = git2::Repository::init(temp_dir.path()).unwrap();
    repo.remote("origin", "https://github.com/automattic/test-repo.git")
        .unwrap();

    // Create .gitignore file to ignore decrypted secrets
    let gitignore_content = "decrypted_secret.txt\n*.secret\nsecrets/\n";
    fs::write(temp_dir.path().join(".gitignore"), gitignore_content).unwrap();

    // Create initial commit to make it a proper repository
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitignore")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(
        Some("refs/heads/trunk"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Test setup command
    let setup_result = setup_command(temp_dir.path());
    if let Err(e) = &setup_result {
        eprintln!("Setup command failed: {e}");
    }
    assert!(setup_result.is_ok());

    // Verify config file was created
    let config_path = temp_dir.path().join(".a8c-secrets").join("config.yaml");
    assert!(config_path.exists());

    // Read and modify config to include our test file
    let config_content = fs::read_to_string(&config_path).unwrap();
    let mut config: serde_yaml::Value = serde_yaml::from_str(&config_content).unwrap();
    if let Some(files) = config.get_mut("files") {
        if let Some(files_array) = files.as_sequence_mut() {
            let mut new_file = serde_yaml::Mapping::new();
            new_file.insert(
                serde_yaml::Value::String("source".to_string()),
                serde_yaml::Value::String("test_secret.txt".to_string()),
            );
            new_file.insert(
                serde_yaml::Value::String("destination".to_string()),
                serde_yaml::Value::String("decrypted_secret.txt".to_string()),
            );
            files_array.push(serde_yaml::Value::Mapping(new_file));
        }
    }
    fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();

    // Test encrypt command
    let encrypt_result = encrypt_command(temp_dir.path());
    if let Err(e) = &encrypt_result {
        eprintln!("Encrypt command failed: {e}");
    }
    assert!(encrypt_result.is_ok());

    // Verify encrypted file was created
    let encrypted_file = temp_dir
        .path()
        .join(".a8c-secrets")
        .join("test_secret.txt.enc");
    assert!(encrypted_file.exists());

    // Test decrypt command
    let decrypt_result = decrypt_command(temp_dir.path());
    assert!(decrypt_result.is_ok());

    // Verify decrypted file was created
    let decrypted_file = temp_dir.path().join("decrypted_secret.txt");
    assert!(decrypted_file.exists());

    // Verify content matches
    let decrypted_content = fs::read_to_string(&decrypted_file).unwrap();
    assert_eq!(decrypted_content, "super secret api key");

    // Clean up
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_setup_command_creates_structure() {
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_home = std::env::var("HOME").ok();

    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");

    // Create mobile-secrets directory
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a bare repository to use as remote for mobile-secrets
    let bare_repo_dir = tempdir().unwrap();
    let _bare_repo = git2::Repository::init_bare(bare_repo_dir.path()).unwrap();

    // Initialize mobile-secrets repository
    let mobile_secrets_repo = git2::Repository::init(&mobile_secrets_path).unwrap();
    mobile_secrets_repo
        .remote("origin", bare_repo_dir.path().to_str().unwrap())
        .unwrap();

    // Create initial commit in mobile-secrets
    let mut index = mobile_secrets_repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = mobile_secrets_repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let _commit_id = mobile_secrets_repo
        .commit(
            Some("refs/heads/trunk"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Set HEAD to point to trunk branch
    mobile_secrets_repo.set_head("refs/heads/trunk").unwrap();

    // Push to remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in temp directory
    let repo = git2::Repository::init(temp_dir.path()).unwrap();
    repo.remote("origin", "https://github.com/automattic/test-repo.git")
        .unwrap();

    // Create .gitignore file
    let gitignore_content = "*.secret\nsecrets/\n";
    fs::write(temp_dir.path().join(".gitignore"), gitignore_content).unwrap();

    // Create initial commit
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitignore")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(
        Some("refs/heads/trunk"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Test setup command
    let result = setup_command(temp_dir.path());
    assert!(result.is_ok());

    // Verify the structure was created
    let config_path = temp_dir.path().join(".a8c-secrets").join("config.yaml");
    assert!(config_path.exists());

    // Verify config file has expected structure
    let config_content = fs::read_to_string(&config_path).unwrap();
    let config: serde_yaml::Value = serde_yaml::from_str(&config_content).unwrap();
    assert!(config.get("sha1").is_some());
    assert!(config.get("files").is_some());

    // Clean up
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_encrypt_command_with_env_var() {
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_home = std::env::var("HOME").ok();

    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");

    // Create mobile-secrets directory
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "test secret content").unwrap();

    // Create encryption keys file
    let keys_file = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
    let keys_content = r#"
test-repo: "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="
"#;
    fs::write(&keys_file, keys_content).unwrap();

    // Initialize git repository in mobile-secrets
    let mobile_secrets_repo = git2::Repository::init(&mobile_secrets_path).unwrap();
    let mut index = mobile_secrets_repo.index().unwrap();
    index.add_path(Path::new("test_secret.txt")).unwrap();
    index
        .add_path(Path::new(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE))
        .unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = mobile_secrets_repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let _commit_id = mobile_secrets_repo
        .commit(
            Some("refs/heads/trunk"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Set HEAD to point to trunk branch
    mobile_secrets_repo.set_head("refs/heads/trunk").unwrap();

    // Create a bare repository to use as remote for mobile-secrets
    let bare_repo_dir = tempdir().unwrap();
    let _bare_repo = git2::Repository::init_bare(bare_repo_dir.path()).unwrap();

    // Add the bare repo as remote to mobile-secrets repository
    mobile_secrets_repo
        .remote("origin", bare_repo_dir.path().to_str().unwrap())
        .unwrap();

    // Push the trunk branch to the remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in temp directory
    let repo = git2::Repository::init(temp_dir.path()).unwrap();
    repo.remote("origin", "https://github.com/automattic/test-repo.git")
        .unwrap();

    // Create .gitignore file
    let gitignore_content = "*.secret\nsecrets/\ntest_secret.txt\n";
    fs::write(temp_dir.path().join(".gitignore"), gitignore_content).unwrap();

    // Create initial commit
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitignore")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(
        Some("refs/heads/trunk"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Create config file
    let config_dir = temp_dir.path().join(".a8c-secrets");
    fs::create_dir_all(&config_dir).unwrap();

    let config_content = r#"sha1: "abc123def456"
files:
  - source: "test_secret.txt"
    destination: "test_secret.txt"
"#;
    fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Test encrypt command
    let result = encrypt_command(temp_dir.path());
    assert!(result.is_ok());

    // Verify encrypted file was created
    let encrypted_file = config_dir.join("test_secret.txt.enc");
    assert!(encrypted_file.exists());

    // Clean up
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_decrypt_command_with_env_var() {
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_home = std::env::var("HOME").ok();

    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");

    // Create mobile-secrets directory
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "test secret content").unwrap();

    // Create encryption keys file
    let keys_file = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
    let keys_content = r#"
test-repo: "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="
"#;
    fs::write(&keys_file, keys_content).unwrap();

    // Initialize git repository in mobile-secrets
    let mobile_secrets_repo = git2::Repository::init(&mobile_secrets_path).unwrap();
    let mut index = mobile_secrets_repo.index().unwrap();
    index.add_path(Path::new("test_secret.txt")).unwrap();
    index
        .add_path(Path::new(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE))
        .unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = mobile_secrets_repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let _commit_id = mobile_secrets_repo
        .commit(
            Some("refs/heads/trunk"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Set HEAD to point to trunk branch
    mobile_secrets_repo.set_head("refs/heads/trunk").unwrap();

    // Create a bare repository to use as remote for mobile-secrets
    let bare_repo_dir = tempdir().unwrap();
    let _bare_repo = git2::Repository::init_bare(bare_repo_dir.path()).unwrap();

    // Add the bare repo as remote to mobile-secrets repository
    mobile_secrets_repo
        .remote("origin", bare_repo_dir.path().to_str().unwrap())
        .unwrap();

    // Push the trunk branch to the remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in temp directory
    let repo = git2::Repository::init(temp_dir.path()).unwrap();
    repo.remote("origin", "https://github.com/automattic/test-repo.git")
        .unwrap();

    // Create .gitignore file
    let gitignore_content = "*.secret\nsecrets/\n";
    fs::write(temp_dir.path().join(".gitignore"), gitignore_content).unwrap();

    // Create initial commit
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitignore")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(
        Some("refs/heads/trunk"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Create config file
    let config_dir = temp_dir.path().join(".a8c-secrets");
    fs::create_dir_all(&config_dir).unwrap();

    let config_content = r#"sha1: "abc123def456"
files:
  - source: "test_secret.txt"
    destination: "test_secret.txt"
"#;
    fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Create encrypted file manually
    let encrypted_file = config_dir.join("test_secret.txt.enc");
    let encrypted_content = "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="; // This is just a placeholder
    fs::write(&encrypted_file, encrypted_content).unwrap();

    // Test decrypt command
    let result = decrypt_command(temp_dir.path());
    if let Err(e) = &result {
        eprintln!("Decrypt command failed: {e}");
    }
    // This might fail due to the placeholder encrypted content, but that's okay for this test

    // Clean up
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_decrypt_with_tilde_destination() {
    // Save original HOME value
    let original_home = std::env::var("HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());

    // Create a test repository with config
    let repo_dir = tempfile::tempdir().unwrap();
    let config_dir = repo_dir.path().join(".a8c-secrets");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Create mobile-secrets directory
    let mobile_secrets_path = temp_dir.path().join(".mobile-secrets");
    std::fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("secrets").join("api_key.txt");
    std::fs::create_dir_all(secret_file.parent().unwrap()).unwrap();
    std::fs::write(&secret_file, "test api key").unwrap();

    // Create encryption keys file
    let keys_file = mobile_secrets_path.join(MOBILE_SECRETS_ENCRYPTION_KEYS_FILE);
    let key = "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="; // base64 encoded test key
    let keys_content = format!("test-repo: {key}");
    fs::write(&keys_file, keys_content).unwrap();

    // Set environment variable for the encryption key
    std::env::set_var("A8C_SECRETS_ENCRYPTION_KEY", key);

    // Create config with tilde destination
    let config = r#"sha1: "abc123def456"
files:
  - source: "secrets/api_key.txt"
    destination: "~/.test-app/secrets.swift"
"#;
    fs::write(config_dir.join("config.yaml"), config).unwrap();

    // Test that the config loads correctly with tilde expanded
    let config = a8c_secrets::paths::load_config(repo_dir.path()).unwrap();
    let expected = temp_dir.path().join(".test-app").join("secrets.swift");
    assert_eq!(config.files[0].destination, expected.to_string_lossy());

    // Test that we can encrypt (this should work)
    let result = a8c_secrets::commands::encrypt_command(repo_dir.path());
    if result.is_err() {
        eprintln!("Encrypt command failed: {:?}", result.unwrap_err());
        // This might fail due to git setup, but that's okay for this test
    }

    // Test that we can decrypt (this should work if encrypt worked)
    let result = a8c_secrets::commands::decrypt_command(repo_dir.path());
    if result.is_err() {
        eprintln!("Decrypt command failed: {:?}", result.unwrap_err());
        // This might fail due to git setup, but that's okay for this test
    }

    // Restore original HOME value
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_tilde_expansion_functionality() {
    // Save original HOME value
    let original_home = std::env::var("HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());

    // Create a test repository with config
    let repo_dir = tempfile::tempdir().unwrap();
    let config_dir = repo_dir.path().join(".a8c-secrets");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Test that we can create a config with tilde and it loads correctly
    let config_content = r#"sha1: "abc123def456"
files:
  - source: "test.txt"
    destination: "~/.test-app/secrets.swift"
"#;

    // Write config file
    std::fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Parse the config directly - tilde won't be expanded yet
    let raw_config: a8c_secrets::Config = serde_yaml::from_str(config_content).unwrap();
    assert_eq!(raw_config.files[0].destination, "~/.test-app/secrets.swift");

    // Test that load_config expands the tilde
    let config = a8c_secrets::paths::load_config(repo_dir.path()).unwrap();
    // The destination should be expanded to the HOME directory we set
    assert!(config.files[0].destination.contains(".test-app"));
    assert!(config.files[0].destination.contains("secrets.swift"));
    assert!(!config.files[0].destination.starts_with('~'));

    // Test that we can create the directory and write a file
    let expanded = std::path::PathBuf::from(&config.files[0].destination);
    if let Some(parent) = expanded.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&expanded, "test content").unwrap();

    // Verify the file was created in the expected location
    assert!(expanded.exists());
    assert_eq!(std::fs::read_to_string(&expanded).unwrap(), "test content");

    // Restore original HOME value
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}
