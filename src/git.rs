use anyhow::{anyhow, Result};
use std::path::Path;

/// Gets the SHA1 hash of the current HEAD commit in the ~/.mobile-secrets repository.
///
/// # Arguments
/// - `mobile_secrets_path` - Path to the ~/.mobile-secrets directory
///
/// # Returns
/// - `Ok(String)` containing the SHA1 hash of the HEAD commit
/// - `Err(anyhow::Error)` if the repository can't be opened or HEAD can't be resolved
pub fn get_mobile_secrets_head_sha1(mobile_secrets_path: &Path) -> Result<String> {
    let repo = git2::Repository::open(mobile_secrets_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Extracts the repository name from the current directory's git remote origin URL.
///
/// # Returns
/// - `Ok(String)` containing the repository name (e.g., "my-repo" from "git@github.com:user/my-repo.git")
/// - `Err(anyhow::Error)` if git repository can't be opened, origin remote doesn't exist, or URL is invalid
pub fn get_current_repo_name() -> Result<String> {
    let repo = git2::Repository::open(".")?;
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().ok_or_else(|| anyhow!("No remote URL found"))?;
    
    // Extract repository name from URL (handles both SSH and HTTPS URLs, removing .git suffix)
    let url = remote_url.trim_end_matches(".git");
    let name = url
        .split('/')
        .next_back()
        .ok_or_else(|| anyhow!("Could not extract repo name from URL"))?;
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
    let head_commit = head.peel_to_commit()?;
    
    // Get the remote trunk commit
    let trunk_ref = repo.find_reference("refs/remotes/origin/trunk")?;
    let trunk_commit = trunk_ref.peel_to_commit()?;
    
    // Check if we're behind the remote trunk
    let commits_behind = repo.graph_ahead_behind(head_commit.id(), trunk_commit.id())?.1;
    
    if commits_behind > 0 {
        println!("⚠️  Warning: Your ~/.mobile-secrets repository is {commits_behind} commit(s) behind origin/trunk.");
        println!("   This means you might be encrypting outdated secrets.");
        println!();
        println!("   To update to the latest version, run:");
        println!("   cd ~/.mobile-secrets && git checkout trunk && git pull");
        println!();
        
        // Ask user if they want to continue
        println!("   Do you want to continue with the current version? (y/N)");
        
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                println!("   Continuing with current version...");
                return Ok(());
            }
            return Err(anyhow!(
                "Update cancelled. Please update ~/.mobile-secrets first:\n\
                cd ~/.mobile-secrets && git checkout trunk && git pull"
            ));
        }
        return Err(anyhow!("Failed to read user input. Please update ~/.mobile-secrets first."));
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
pub fn ensure_destination_is_git_ignored(destination_path: &str) -> Result<()> {
    let repo = git2::Repository::open(".").map_err(|e| {
        anyhow!(
            "Failed to open git repository: {}\n\n\
            Make sure you're running this command from within a git repository.",
            e
        )
    })?;

    let is_ignored = repo.is_path_ignored(Path::new(destination_path)).map_err(|e| {
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
            destination_path, destination_path, destination_path, destination_path
        ))
    }
}
