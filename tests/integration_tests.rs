use a8c_secrets::{crypto, decrypt_command, encrypt_command, setup_command};
use base64::Engine;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_full_workflow() {
    // Create temporary directories
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_dir = std::env::current_dir().unwrap();
    let original_home = std::env::var("HOME").ok();

    std::env::set_current_dir(&temp_dir).unwrap();
    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");

    // Create mobile-secrets directory structure
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "super secret api key").unwrap();

    // Create encryption keys file
    let keys_file = mobile_secrets_path.join("encryption-keys.yaml");
    let keys_content = r#"
repos:
  test-repo:
    key: "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI="
"#;
    fs::write(&keys_file, keys_content).unwrap();

    // Initialize git repository in mobile-secrets
    let mobile_secrets_repo = git2::Repository::init(&mobile_secrets_path).unwrap();
    let mut index = mobile_secrets_repo.index().unwrap();
    index.add_path(Path::new("test_secret.txt")).unwrap();
    index.add_path(Path::new("encryption-keys.yaml")).unwrap();
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

    // Initialize git repository in current directory
    let repo = git2::Repository::init(&temp_dir).unwrap();
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
    let setup_result = setup_command();
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
    let encrypt_result = encrypt_command();
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
    let decrypt_result = decrypt_command();
    assert!(decrypt_result.is_ok());

    // Verify decrypted file was created
    let decrypted_file = temp_dir.path().join("decrypted_secret.txt");
    assert!(decrypted_file.exists());

    // Verify content matches
    let decrypted_content = fs::read_to_string(&decrypted_file).unwrap();
    assert_eq!(decrypted_content, "super secret api key");

    // Clean up
    std::env::set_current_dir(original_dir).unwrap();
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
    let original_dir = std::env::current_dir().unwrap();
    let original_home = std::env::var("HOME").ok();

    std::env::set_current_dir(&temp_dir).unwrap();
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

    // Initialize git repository in current directory
    let repo = git2::Repository::init(&temp_dir).unwrap();
    repo.remote("origin", "https://github.com/automattic/test-repo.git")
        .unwrap();

    // Create initial commit to make it a proper repository
    let mut index = repo.index().unwrap();
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
    let result = setup_command();
    assert!(result.is_ok());

    // Verify directory structure was created
    let config_dir = temp_dir.path().join(".a8c-secrets");
    assert!(config_dir.exists());
    assert!(config_dir.is_dir());

    let config_file = config_dir.join("config.yaml");
    assert!(config_file.exists());
    assert!(config_file.is_file());

    // Verify config file has correct structure
    let config_content = fs::read_to_string(&config_file).unwrap();
    let config: serde_yaml::Value = serde_yaml::from_str(&config_content).unwrap();
    assert!(config.get("files").is_some());

    // Clean up
    std::env::set_current_dir(original_dir).unwrap();
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
    let original_dir = std::env::current_dir().unwrap();
    let original_home = std::env::var("HOME").ok();

    std::env::set_current_dir(&temp_dir).unwrap();
    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");
    std::env::set_var(
        "A8C_SECRETS_ENCRYPTION_KEY",
        "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI=",
    );

    // Create mobile-secrets directory structure
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "super secret api key").unwrap();

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

    // Push to remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in current directory
    let repo = git2::Repository::init(&temp_dir).unwrap();
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

    // Create config file
    let config_dir = temp_dir.path().join(".a8c-secrets");
    fs::create_dir_all(&config_dir).unwrap();

    let config_content = r#"
sha1: "abc123def456"
files:
  - source: "test_secret.txt"
    destination: "decrypted_secret.txt"
"#;
    fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Test encrypt command
    let result = encrypt_command();
    assert!(result.is_ok());

    // Verify encrypted file was created
    let encrypted_file = temp_dir
        .path()
        .join(".a8c-secrets")
        .join("test_secret.txt.enc");
    assert!(encrypted_file.exists());

    // Clean up
    std::env::set_current_dir(original_dir).unwrap();
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    std::env::remove_var("A8C_SECRETS_ENCRYPTION_KEY");
}

#[test]
fn test_decrypt_command_with_env_var() {
    let temp_dir = tempdir().unwrap();
    let mobile_secrets_dir = tempdir().unwrap();

    // Set up environment
    let original_dir = std::env::current_dir().unwrap();
    let original_home = std::env::var("HOME").ok();

    std::env::set_current_dir(&temp_dir).unwrap();
    std::env::set_var("HOME", mobile_secrets_dir.path());
    std::env::set_var("A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND", "yes");
    std::env::set_var(
        "A8C_SECRETS_ENCRYPTION_KEY",
        "dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI=",
    );

    // Create mobile-secrets directory structure
    let mobile_secrets_path = mobile_secrets_dir.path().join(".mobile-secrets");
    fs::create_dir_all(&mobile_secrets_path).unwrap();

    // Create a test secret file
    let secret_file = mobile_secrets_path.join("test_secret.txt");
    fs::write(&secret_file, "super secret api key").unwrap();

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

    // Push to remote
    let mut remote = mobile_secrets_repo.find_remote("origin").unwrap();
    remote
        .push(&["refs/heads/trunk:refs/heads/trunk"], None)
        .unwrap();

    // Initialize git repository in current directory
    let repo = git2::Repository::init(&temp_dir).unwrap();
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

    // Create config file
    let config_dir = temp_dir.path().join(".a8c-secrets");
    fs::create_dir_all(&config_dir).unwrap();

    let config_content = r#"
sha1: "abc123def456"
files:
  - source: "test_secret.txt"
    destination: "decrypted_secret.txt"
"#;
    fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Create encrypted file (simulate previous encrypt)
    let key = base64::engine::general_purpose::STANDARD
        .decode("dGVzdC1rZXktZm9yLXRlc3RpbmctcHVycG9zZXMtMzI=")
        .unwrap();
    let encrypted_content = crypto::encrypt_data(b"super secret api key", &key).unwrap();
    fs::write(
        temp_dir
            .path()
            .join(".a8c-secrets")
            .join("test_secret.txt.enc"),
        encrypted_content,
    )
    .unwrap();

    // Test decrypt command
    let result = decrypt_command();
    if let Err(e) = &result {
        eprintln!("Decrypt command failed: {e}");
    }
    assert!(result.is_ok());

    // Verify decrypted file was created
    let decrypted_file = temp_dir.path().join("decrypted_secret.txt");
    assert!(decrypted_file.exists());

    // Verify content matches
    let decrypted_content = fs::read_to_string(&decrypted_file).unwrap();
    assert_eq!(decrypted_content, "super secret api key");

    // Clean up
    std::env::set_current_dir(original_dir).unwrap();
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    std::env::remove_var("A8C_SECRETS_ENCRYPTION_KEY");
}
