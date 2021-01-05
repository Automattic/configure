use crate::string::distance_between_strings_in;
use crate::Configuration;
use crate::ConfigureError;
use git2::Oid;
use git2::{BranchType, ErrorCode, Repository, ResetType};
use log::debug;

pub struct SecretsRepo {
    pub path: std::path::PathBuf,
}

impl Default for SecretsRepo {
    fn default() -> Self {
        SecretsRepo {
            path: crate::fs::find_secrets_repo().expect("Unable to find secrets repo"),
        }
    }
}

impl SecretsRepo {
    fn get_repo(&self) -> Result<Repository, ConfigureError> {
        Ok(Repository::open(&self.path)?)
    }

    // Assumes you're using `origin` as the remote name
    pub fn update_local_copy(&self) -> Result<(), ConfigureError> {
        debug!("Running `git fetch`");

        std::process::Command::new("git")
            .arg("fetch")
            .current_dir(std::fs::canonicalize(&self.path).unwrap())
            .output()?; // Wait for it to finish and collect its output

        debug!("Fetch Complete");

        Ok(())
    }

    pub fn status(&self) -> Result<RepoStatus, ConfigureError> {
        RepoStatus::from_repo(self)
    }

    pub fn current_branch(&self) -> Result<String, ConfigureError> {
        let repo = self.get_repo()?;
        let head = match repo.head() {
            Ok(head) => Some(head),
            Err(ref e)
                if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound =>
            {
                None
            }
            Err(_) => return Err(ConfigureError::GitGetCurrentBranchError),
        };

        let head = head.as_ref().and_then(|h| h.shorthand());

        Ok(head.unwrap().to_string())
    }

    pub fn current_hash(&self) -> Result<String, ConfigureError> {
        let repo = self.get_repo()?;
        let latest_commit = repo.head()?.peel_to_commit()?;
        Ok(latest_commit.id().to_string())
    }

    // TODO: I don't think this works right?
    pub fn latest_local_hash_for_branch(
        &self,
        branch_name: &str,
    ) -> Result<String, ConfigureError> {
        let current_branch = self.current_branch()?;

        if current_branch != branch_name {
            self.switch_to_branch(branch_name)?
        }

        let repo = self.get_repo()?;
        let latest_commit = repo.head()?.peel_to_commit()?;

        Ok(latest_commit.id().to_string())
    }

    pub fn latest_remote_hash_for_branch(
        &self,
        branch_name: &str,
    ) -> Result<String, ConfigureError> {
        let remote_ref = "origin/".to_owned() + branch_name;

        debug!("Looking for remote ref: {:?}", remote_ref);

        let output = std::process::Command::new("git")
            .arg("rev-parse")
            .arg(remote_ref)
            .current_dir(std::fs::canonicalize(&self.path).unwrap())
            .output()?; // Wait for it to finish and collect its output

        let string = std::str::from_utf8(&output.stdout).expect("Unable to parse output");

        debug!("Result: {}", string);

        Ok(String::from(string.trim_end()))
    }

    pub fn checkout_local_hash(&self, hash: &str) -> Result<(), ConfigureError> {
        let repo = SecretsRepo::default().get_repo()?;

        let oid = Oid::from_str(hash)?;

        let obj = repo.find_commit(oid)?.into_object();

        repo.set_head_detached(oid)?;
        repo.reset(&obj, ResetType::Hard, None)?;

        Ok(())
    }

    pub fn switch_to_branch(&self, branch_name: &str) -> Result<(), ConfigureError> {
        debug!("Trying to check out branch: {:?}", branch_name);

        let ref_name = "refs/heads/".to_owned() + branch_name;
        debug!("Checking out: {:?}", ref_name);

        self.get_repo()?.set_head(&ref_name)?;

        debug!("Checkout successful");
        Ok(())
    }

    pub fn switch_to_branch_at_revision(
        &self,
        branch_name: &str,
        revision: &str,
    ) -> Result<(), ConfigureError> {
        // If we're asked to check out a commit that's not currently on a branch,
        // just switch to it directly
        if branch_name == "HEAD" {
            return self.checkout_local_hash(revision);
        }

        let repo = self.get_repo()?;
        let ref_name = "refs/heads/".to_owned() + branch_name;

        repo.set_head(&ref_name)?;

        let oid = Oid::from_str(revision)?;

        let obj = repo.find_commit(oid)?.into_object();

        repo.reset(&obj, ResetType::Hard, None)?;

        Ok(())
    }

    pub fn local_branch_names(&self) -> Result<Vec<String>, ConfigureError> {
        let repo = self.get_repo()?;
        let branches = repo.branches(Some(BranchType::Local))?;
        let branch_names: Vec<String> = branches
            .into_iter()
            .map(|branch| branch.expect("Unable to read branch"))
            .map(|branch| {
                String::from(
                    branch
                        .0
                        .name()
                        .expect("Unable to read branch name")
                        .unwrap(),
                )
            })
            .collect::<Vec<String>>();

        Ok(branch_names)
    }

    /// How far out of date the configure file is relative to the secrets repo
    pub fn commits_ahead_of_configuration(&self, configuration: &Configuration) -> i32 {
        let current_branch = self
            .current_branch()
            .expect("Unable to get current mobile secrets branch");
        let current_hash = self
            .current_hash()
            .expect("Unable to get current mobile secrets hash");

        self.switch_to_branch(&configuration.branch)
            .expect("Unable to switch branches – you might need to fetch the most recent changes from the remote first?");

        let latest_hash = self
            .current_hash()
            .expect("Unable to retrieve current secrets hash");

        let distance = self
            .distance_between_local_commit_hashes(&configuration.pinned_hash, &latest_hash)
            .expect("Unable to determine the distance between two hashes");

        // Restore the secrets repo to its state before starting
        self.switch_to_branch_at_revision(&current_branch, &current_hash)
            .expect("Unable to roll back to branch");

        distance
    }

    // Returns the number of commits between two hashes. If the hashes aren't part of the same history
    // or if `hash2` comes before `hash1`, the result will be `0`
    fn distance_between_local_commit_hashes(
        &self,
        hash1: &str,
        hash2: &str,
    ) -> Result<i32, ConfigureError> {
        // If we're asked to calculate the distance between two of the same hash, we can skip a lot of work
        if hash1 == hash2 {
            debug!("Hashes are identical – skipping checks");
            return Ok(0);
        }

        let hash_list = self.get_hash_list()?;

        match distance_between_strings_in(hash1, hash2, &hash_list) {
            Some(distance) => Ok(distance),
            None => Err(ConfigureError::GitStatusUnknownError),
        }
    }

    fn get_hash_list(&self) -> Result<Vec<String>, std::io::Error> {
        debug!("Opening secrets repo at {:?}", self.path);

        let output = std::process::Command::new("git")
            .arg("--no-pager")
            .arg("log")
            .arg("-10000")
            .arg("--pretty=format:%H")
            .current_dir(std::fs::canonicalize(&self.path).unwrap())
            .output()?;

        debug!("Fetched hash list");

        let lines: Vec<String> = std::str::from_utf8(&output.stdout)
            .expect("Unable to read hash list")
            .lines()
            .rev()
            .map(|s| s.to_string())
            .collect();

        debug!("Hash list has {:} entries:", lines.len());

        Ok(lines)
    }
}

#[derive(Debug)]
pub enum RepoSyncState {
    /// The local secrets repository has commits that the server does not have
    Ahead,

    /// The server has commits that the local secrets repository does not have
    Behind,

    /// The local secrets repository and server are in sync
    Synced,
}

#[derive(Debug)]
pub struct RepoStatus {
    /// The local repository sync state – ahead of, behind, or in sync with the server
    pub sync_state: RepoSyncState,

    /// How many commits the local repository is out of sync by. If the repository is in sync,
    /// this value will be `0`
    pub distance: i32,
}

impl RepoStatus {
    fn synced() -> RepoStatus {
        RepoStatus {
            sync_state: RepoSyncState::Synced,
            distance: 0,
        }
    }

    fn from_repo(repo: &SecretsRepo) -> Result<RepoStatus, ConfigureError> {
        let output = std::process::Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .arg("-b")
            .current_dir(std::fs::canonicalize(&repo.path).unwrap())
            .output()?; // Wait for it to finish and collect its output

        let status = std::str::from_utf8(&output.stdout).expect("Unable to read output data");

        if status.contains("...") {
            return Ok(RepoStatus::synced());
        }

        let digits = status
            .chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<i32>()?;

        if status.contains("ahead") {
            return Ok(RepoStatus {
                sync_state: RepoSyncState::Ahead,
                distance: digits,
            });
        }

        if status.contains("behind") {
            return Ok(RepoStatus {
                sync_state: RepoSyncState::Behind,
                distance: digits,
            });
        }

        Err(ConfigureError::GitStatusUnknownError {})
    }
}
