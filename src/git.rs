use anyhow::{anyhow, Result};
use std::path::Path;

/// Environment variable to skip prompts in tests
const SKIP_PROMPT_ENV_VAR: &str = "A8C_SECRETS_SKIP_PROMPT_IF_MOBILE_SECRETS_BEHIND";

/// Validates that the repository is not in a detached HEAD state.
///
/// This function checks if the HEAD reference is a symbolic reference (pointing to a branch)
/// rather than a direct reference (pointing to a specific commit).
///
/// # Arguments
/// - `head` - The HEAD reference to validate
///
/// # Returns
/// - `Ok(())` if HEAD is not detached
/// - `Err(anyhow::Error)` if HEAD is detached
fn validate_not_detached_head(head: &git2::Reference) -> Result<()> {
    if head.kind() == Some(git2::ReferenceType::Direct) {
        // Check if HEAD is pointing to a branch reference (e.g., refs/heads/trunk)
        // This can happen with git2 even when properly checked out
        if let Some(name) = head.name() {
            if name.starts_with("refs/heads/") {
                // This is actually a branch reference, not truly detached
                return Ok(());
            }
        }

        return Err(anyhow!(
            "The ~/.mobile-secrets repository is in a detached HEAD state.\n\n\
            This usually happens when you've checked out a specific commit instead of a branch.\n\n\
            To fix this, run:\n\
            cd ~/.mobile-secrets && git checkout trunk"
        ));
    }
    Ok(())
}

/// Gets the SHA1 hash of the current HEAD commit in the ~/.mobile-secrets repository.
///
/// # Arguments
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(String)` containing the SHA1 hash of the HEAD commit
/// - `Err(anyhow::Error)` if the repository can't be opened, HEAD can't be resolved, or repository is in detached HEAD state
pub fn get_mobile_secrets_head_sha1(mobile_secrets_path: &Path) -> Result<String> {
    let repo = git2::Repository::open(mobile_secrets_path)?;
    let head = repo.head()?;

    // Check if we're in a detached HEAD state
    validate_not_detached_head(&head)?;

    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Extracts the repository name from the specified directory's git remote origin URL.
///
/// # Arguments
/// - `repo_path` - Path to the git repository directory
///
/// # Returns
/// - `Ok(String)` containing the repository name (e.g., "my-repo" from "git@github.com:user/my-repo.git")
/// - `Err(anyhow::Error)` if git repository can't be opened, origin remote doesn't exist, or URL is invalid
pub fn get_repo_name(repo_path: &Path) -> Result<String> {
    let repo = git2::Repository::open(repo_path)?;
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().ok_or_else(|| {
        anyhow!(
            "No remote URL found for origin remote.\n\n\
            Make sure your repository has an origin remote configured:\n\
            git remote add origin <repository-url>"
        )
    })?;

    // Extract repository name from URL (handles both SSH and HTTPS URLs, removing .git suffix)
    let url = remote_url.trim_end_matches(".git");
    let name = url.split('/').next_back().ok_or_else(|| {
        anyhow!(
            "Could not extract repository name from URL: {}\n\n\
                The URL should be in the format:\n\
                - git@github.com:user/repo-name.git\n\
                - https://github.com/user/repo-name.git",
            remote_url
        )
    })?;
    Ok(name.to_owned())
}

/// Checks if the mobile-secrets repository is up-to-date with its remote.
///
/// This function checks if the local ~/.mobile-secrets repository is behind its remote
/// and provides a warning to the user if updates are available.
///
/// # Arguments
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(())` if the repository is up-to-date or the user chooses to continue
/// - `Err(anyhow::Error)` if the user chooses to abort or there's an error
pub fn check_mobile_secrets_up_to_date(mobile_secrets_path: &Path) -> Result<()> {
    let repo = git2::Repository::open(mobile_secrets_path)?;

    // Fetch the latest changes from remote
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["trunk"], None, None)?;

    // Get the current HEAD commit
    let head = repo.head()?;

    // Check if we're in a detached HEAD state
    validate_not_detached_head(&head)?;

    let head_commit = head.peel_to_commit()?;

    // Get the remote trunk commit
    let trunk_ref = repo.find_reference("refs/remotes/origin/trunk")?;
    let trunk_commit = trunk_ref.peel_to_commit()?;

    // Check if we're behind the remote trunk
    let commits_behind = repo
        .graph_ahead_behind(head_commit.id(), trunk_commit.id())?
        .1;

    if commits_behind > 0 {
        println!("⚠️  Warning: Your ~/.mobile-secrets repository is {commits_behind} commit(s) behind origin/trunk.");
        println!("   This means you might be encrypting outdated secrets.");
        println!();
        println!("   To update to the latest version, run:");
        println!("   cd ~/.mobile-secrets && git checkout trunk && git pull");
        println!();

        // Check if we should skip the prompt (useful for tests)
        let user_wants_to_continue = if let Ok(value) = std::env::var(SKIP_PROMPT_ENV_VAR) {
            let value = value.to_lowercase();
            value != "no" && value != "false"
        } else {
            // Ask user if they want to continue
            println!("   Do you want to continue with the current version? (y/N)");

            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim().to_lowercase();
                input == "y" || input == "yes"
            } else {
                return Err(anyhow!("Failed to read user input"));
            }
        };

        if user_wants_to_continue {
            println!("   Continuing with current version...");
            return Ok(());
        }
        return Err(anyhow!(
            "Update cancelled. Please update ~/.mobile-secrets first:\n\
            cd ~/.mobile-secrets && git checkout trunk && git pull"
        ));
    }

    Ok(())
}

/// Ensures that a destination file path is ignored by git.
///
/// This function checks if the destination file would be ignored by git to prevent
/// accidentally committing decrypted secret files to version control.
///
/// # Arguments
/// - `destination_path` - The path to the destination file to validate
///
/// # Returns
/// - `Ok(())` if the file is properly ignored
/// - `Err(anyhow::Error)` if the file is not ignored or validation fails
pub fn ensure_destination_is_git_ignored(destination_path: &str, repo_path: &Path) -> Result<()> {
    let repo = git2::Repository::open(repo_path).map_err(|e| {
        anyhow!(
            "Failed to open git repository: {}\n\n\
            Make sure you're running this command from within a git repository.",
            e
        )
    })?;

    let is_ignored = repo
        .is_path_ignored(Path::new(destination_path))
        .map_err(|e| {
            anyhow!(
                "Failed to check if path '{}' is ignored by git: {}\n\n\
            This could indicate a problem with the git repository or the file path.",
                destination_path,
                e
            )
        })?;

    if is_ignored {
        // File is properly ignored, continue
        Ok(())
    } else {
        Err(anyhow!(
            "⚠️  SECURITY ERROR: Destination file '{}' is NOT ignored by git!\n\n\
            Refusing to create decrypted secret file that could be accidentally committed.\n\n\
            To fix this issue:\n\
            1. Add '{}' to your .gitignore file, OR\n\
            2. Change the destination path to a location outside your repository, OR\n\
            3. Change the destination path to a location that's already git-ignored\n\n\
            Example .gitignore entries:\n\
            # Ignore this specific file\n\
            {}\n\
            # Or ignore all secret files in a directory\n\
            secrets/\n\
            *.secret\n\n\
            After updating .gitignore, run 'git check-ignore {}' to verify it's ignored.",
            destination_path,
            destination_path,
            destination_path,
            destination_path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper function to create a commit with a dummy file in a repository.
    ///
    /// # Arguments
    /// - `repo` - The git repository to create the commit in
    /// - `file_name` - The name of the dummy file to create
    /// - `file_content` - The content of the dummy file
    /// - `commit_message` - The commit message
    /// - `parents` - Optional parent commits (empty slice for initial commit)
    ///
    /// # Returns
    /// - `git2::Oid` - The commit ID
    fn create_commit_with_file(
        repo: &git2::Repository,
        file_name: &str,
        file_content: &[u8],
        commit_message: &str,
        parents: &[&git2::Commit],
    ) -> git2::Oid {
        // Create the file content as a blob
        let blob_id = repo.blob(file_content).unwrap();

        // Create a tree with the file
        let mut tree_builder = repo.treebuilder(None).unwrap();
        tree_builder.insert(file_name, blob_id, 0o100_644).unwrap();
        let tree_id = tree_builder.write().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        // Create the commit
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        let commit_id = repo
            .commit(
                Some("refs/heads/trunk"),
                &signature,
                &signature,
                commit_message,
                &tree,
                parents,
            )
            .unwrap();

        // Drop borrowed objects
        drop(tree);
        drop(signature);

        commit_id
    }

    /// Helper function to create a git repository with an initial commit and optional remote.
    ///
    /// # Arguments
    /// - `remote_url` - Optional remote URL. If None, no remote is added.
    ///
    /// # Returns
    /// - `(tempfile::TempDir, git2::Repository)` - The temp directory and repository
    fn create_test_repo(remote_url: Option<&str>) -> (tempfile::TempDir, git2::Repository) {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        let repo = git2::Repository::init(temp_path).unwrap();

        // Create initial commit with a dummy file
        create_commit_with_file(&repo, "test.txt", b"test content", "Initial commit", &[]);
        repo.set_head("refs/heads/trunk").unwrap();

        // Add remote if provided
        if let Some(url) = remote_url {
            repo.remote("origin", url).unwrap();
        }

        (temp_dir, repo)
    }

    /// Helper function to create a test repository with a bare repository as its remote.
    ///
    /// # Returns
    /// - `(tempfile::TempDir, tempfile::TempDir)` - The working copy directory and bare repo directory
    fn create_test_repo_with_bare_remote() -> (tempfile::TempDir, tempfile::TempDir) {
        // Create a bare repository to serve as the remote
        let bare_repo_dir = tempdir().unwrap();
        let bare_repo = git2::Repository::init_bare(bare_repo_dir.path()).unwrap();

        // Create initial commit in the bare repo on trunk branch
        create_commit_with_file(
            &bare_repo,
            "initial.txt",
            b"initial content",
            "Initial commit",
            &[],
        );

        // Create the working copy (mobile-secrets repo)
        let working_dir = tempdir().unwrap();
        let working_repo = git2::Repository::init(working_dir.path()).unwrap();

        // Add the bare repo as remote
        working_repo
            .remote("origin", bare_repo_dir.path().to_str().unwrap())
            .unwrap();

        // Fetch from remote to get the trunk branch
        let mut remote = working_repo.find_remote("origin").unwrap();
        remote.fetch(&["trunk"], None, None).unwrap();

        // Use git2 to checkout the trunk branch
        let trunk_oid = working_repo
            .refname_to_id("refs/remotes/origin/trunk")
            .unwrap();
        let trunk_commit = working_repo.find_commit(trunk_oid).unwrap();

        // Create a local trunk branch pointing to the remote trunk
        working_repo.branch("trunk", &trunk_commit, false).unwrap();

        // Set HEAD to the trunk branch
        working_repo.set_head("refs/heads/trunk").unwrap();

        // Drop borrowed objects
        drop(remote);
        drop(trunk_commit);

        (working_dir, bare_repo_dir)
    }

    #[test]
    fn test_get_repo_name_https_with_git_suffix() {
        let (temp_dir, _repo) =
            create_test_repo(Some("https://github.com/automattic/test-repo.git"));

        let repo_name = get_repo_name(temp_dir.path()).unwrap();
        assert_eq!(repo_name, "test-repo");
    }

    #[test]
    fn test_get_repo_name_https_without_git_suffix() {
        let (temp_dir, _repo) = create_test_repo(Some("https://github.com/automattic/test-repo"));

        let repo_name = get_repo_name(temp_dir.path()).unwrap();
        assert_eq!(repo_name, "test-repo");
    }

    #[test]
    fn test_get_repo_name_ssh_with_git_suffix() {
        let (temp_dir, _repo) = create_test_repo(Some("git@github.com:automattic/test-repo.git"));

        let repo_name = get_repo_name(temp_dir.path()).unwrap();
        assert_eq!(repo_name, "test-repo");
    }

    #[test]
    fn test_get_repo_name_ssh_without_git_suffix() {
        let (temp_dir, _repo) = create_test_repo(Some("git@github.com:automattic/test-repo"));

        let repo_name = get_repo_name(temp_dir.path()).unwrap();
        assert_eq!(repo_name, "test-repo");
    }

    #[test]
    fn test_get_repo_name_with_subdomain() {
        let (temp_dir, _repo) =
            create_test_repo(Some("https://git.example.com/org/my-project.git"));

        let repo_name = get_repo_name(temp_dir.path()).unwrap();
        assert_eq!(repo_name, "my-project");
    }

    #[test]
    fn test_get_repo_name_no_remote() {
        let (temp_dir, _repo) = create_test_repo(None);

        let result = get_repo_name(temp_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Accept any error about missing remote
        assert!(
            error_msg.contains("remote")
                || error_msg.contains("origin")
                || error_msg.contains("find_remote")
        );
    }

    #[test]
    fn test_get_repo_name_not_git_repo() {
        let temp_dir = tempdir().unwrap();
        // Test with a non-git directory
        let result = get_repo_name(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_mobile_secrets_head_sha1_success() {
        let (temp_dir, repo) = create_test_repo(None);

        // Get the commit ID from the repository
        let head = repo.head().unwrap();
        let commit_id = head.peel_to_commit().unwrap().id();

        // The function should work and return the correct SHA1
        let sha1 = get_mobile_secrets_head_sha1(temp_dir.path()).unwrap();
        assert_eq!(sha1, commit_id.to_string());
    }

    #[test]
    fn test_get_mobile_secrets_head_sha1_detached_head() {
        let (temp_dir, repo) = create_test_repo(None);

        // Set HEAD to detached state
        let head = repo.head().unwrap();
        let commit_id = head.peel_to_commit().unwrap().id();
        repo.set_head_detached(commit_id).unwrap();

        let result = get_mobile_secrets_head_sha1(temp_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("detached HEAD") || error_msg.contains("detached HEAD state"));
    }

    #[test]
    fn test_get_mobile_secrets_head_sha1_not_git_repo() {
        let temp_dir = tempdir().unwrap();
        let result = get_mobile_secrets_head_sha1(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_check_mobile_secrets_up_to_date_success() {
        let (working_dir, _bare_repo_dir) = create_test_repo_with_bare_remote();

        // The function should succeed because we're up-to-date with remote trunk
        let result = check_mobile_secrets_up_to_date(working_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_mobile_secrets_up_to_date_behind_remote() {
        let (working_dir, bare_repo_dir) = create_test_repo_with_bare_remote();

        // Now add a new commit to the bare repo (simulating remote update)
        let bare_repo = git2::Repository::open_bare(bare_repo_dir.path()).unwrap();

        // Get the initial commit as parent
        let trunk_ref = bare_repo.find_reference("refs/heads/trunk").unwrap();
        let initial_commit = trunk_ref.peel_to_commit().unwrap();

        // Create a new commit with a new file
        create_commit_with_file(
            &bare_repo,
            "new_file.txt",
            b"new content",
            "New commit",
            &[&initial_commit],
        );

        // Test case 1: User chooses to continue (should succeed)
        std::env::set_var(SKIP_PROMPT_ENV_VAR, "yes"); // Skip prompt and return yes
        let result = check_mobile_secrets_up_to_date(working_dir.path());
        assert!(result.is_ok());

        // Test case 2: User chooses not to continue (should fail)
        std::env::set_var(SKIP_PROMPT_ENV_VAR, "no"); // Skip prompt and return no
        let result = check_mobile_secrets_up_to_date(working_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should contain information about being cancelled
        assert!(error_msg.contains("cancelled") || error_msg.contains("update"));
    }

    #[test]
    fn test_check_mobile_secrets_up_to_date_detached_head() {
        let (working_dir, _bare_repo_dir) = create_test_repo_with_bare_remote();

        // Open the working copy repository and set it to detached HEAD state
        let repo = git2::Repository::open(working_dir.path()).unwrap();
        let head = repo.head().unwrap();
        let commit_id = head.peel_to_commit().unwrap().id();
        repo.set_head_detached(commit_id).unwrap();

        let result = check_mobile_secrets_up_to_date(working_dir.path());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Accept any error about detached HEAD
        assert!(
            error_msg.contains("detached HEAD")
                || error_msg.contains("detached HEAD state")
                || error_msg.contains("HEAD state")
        );
    }

    #[test]
    fn test_check_mobile_secrets_up_to_date_not_git_repo() {
        let temp_dir = tempdir().unwrap();
        let result = check_mobile_secrets_up_to_date(temp_dir.path());
        assert!(result.is_err());
    }
}
