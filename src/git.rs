use crate::ConfigureError;
use git2::Oid;
use git2::{BranchType, Error, ErrorCode, Repository, ResetType};
use log::debug;

trait ProvidesHashList {
    fn get_hash_list() -> Result<Vec<String>, std::io::Error>;
}

pub struct SecretsRepo {
    // pub path: String,
}

impl ProvidesHashList for SecretsRepo {
    fn get_hash_list() -> Result<Vec<String>, std::io::Error> {
        let path = crate::fs::find_secrets_repo()
            .expect("Unable to find secrets repo");

        debug!("Opening secrets repo at {:?}", path);

        let output = std::process::Command::new("git")
            .arg("--no-pager")
            .arg("log")
            .arg("-10000")
            .arg("--pretty=format:%H")
            .current_dir(std::fs::canonicalize(path).unwrap())
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

pub fn get_current_secrets_branch() -> Result<String, Error> {
    let repo = get_secrets_repo()?;
    let head = match repo.head() {
        Ok(head) => Some(head),
        Err(ref e) if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound => {
            None
        }
        Err(e) => return Err(e),
    };

    let head = head.as_ref().and_then(|h| h.shorthand());

    Ok(head.unwrap().to_string())
}

pub fn get_secrets_branches() -> Result<Vec<String>, Error> {
    let repo = get_secrets_repo()?;
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

// Assumes you're using `origin` as the remote name
pub fn fetch_secrets_latest_remote_data() -> Result<(), std::io::Error> {
    let path = crate::fs::find_secrets_repo().unwrap();

    std::process::Command::new("git")
        .arg("fetch")
        .current_dir(std::fs::canonicalize(path).unwrap())
        .output()?; // Wait for it to finish and collect its output

    debug!("Fetch Complete");

    Ok(())
}

pub fn get_secrets_current_hash() -> Result<String, Error> {
    let repo = get_secrets_repo()?;
    let latest_commit = repo.head()?.peel_to_commit()?;
    Ok(latest_commit.id().to_string())
}

// Fetches the latest hash on the specified branch
//
// You should run `fetch_secrets_latest_remote_data` before this method, otherwise your info might be out-of-date
pub fn get_secrets_latest_hash(branch: &str) -> Result<String, Error> {
    if get_current_secrets_branch().unwrap() != branch {}

    let repo = get_secrets_repo()?;
    let latest_commit = repo.head()?.peel_to_commit()?;

    Ok(latest_commit.id().to_string())
}

pub fn get_latest_hash_for_remote_branch(branch: &str) -> Result<String, std::io::Error> {
    let path = crate::fs::find_secrets_repo().unwrap();

    let remote_ref = "origin/".to_owned() + branch;

    debug!("Looking for remote ref: {:?}", remote_ref);

    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg(remote_ref)
        .current_dir(std::fs::canonicalize(path).unwrap())
        .output()?; // Wait for it to finish and collect its output

    let string = std::str::from_utf8(&output.stdout).expect("Unable to parse output");

    debug!("Result: {}", string);

    Ok(String::from(string.trim_end()))
}

pub fn check_out_hash(hash: &str) -> Result<(), Error> {
    let repo = get_secrets_repo()?;

    let oid = Oid::from_str(hash).expect("Invalid Hash");

    let obj = repo
        .find_commit(oid)
        .expect("No commit exists with that hash")
        .into_object();

    repo.set_head_detached(oid)?;
    repo.reset(&obj, ResetType::Hard, None)?;

    Ok(())
}

pub fn check_out_branch(branch_name: &str) -> Result<(), Error> {
    debug!("Trying to check out branch: {:?}", branch_name);
    let repo = get_secrets_repo()?;
    let ref_name = "refs/heads/".to_owned() + branch_name;
    debug!("Checking out: {:?}", ref_name);

    repo.set_head(&ref_name)?;

    debug!("Checkout successful");
    Ok(())
}

pub fn check_out_branch_at_revision(branch_name: &str, hash: &str) -> Result<(), Error> {
    // If we're asked to check out a commit that's not currently on a branch,
    // just switch to it directly
    if branch_name == "HEAD" {
        return check_out_hash(hash);
    }

    let repo = get_secrets_repo()?;
    let ref_name = "refs/heads/".to_owned() + branch_name;

    repo.set_head(&ref_name)?;

    let oid = Oid::from_str(hash).expect("Invalid Hash");

    let obj = repo
        .find_commit(oid)
        .expect("No commit exists with that hash")
        .into_object();

    repo.reset(&obj, ResetType::Hard, None)?;

    Ok(())
}

// Returns the number of commits between two hashes. If the hashes aren't part of the same history
// or if `hash2` comes before `hash1`, the result will be `0`
pub fn secrets_repo_distance_between(hash1: &str, hash2: &str) -> Result<i32, std::io::Error> {
    // If we're asked to calculate the distance between two of the same hash, we can skip a lot of work
    if hash1 == hash2 {
        debug!("Hashes are identical – skipping checks");
        return Ok(0);
    }

    let hash_list = SecretsRepo::get_hash_list()?;

    let index_of_configure_file_hash = hash_list
        .iter()
        .position(|r| r == hash1)
        .unwrap_or_else(|| {
            panic!(
                "The pinned hash in .configure {} doesn't exist in the repository history",
                &hash1
            )
        });

    debug!("Configure file hash is at position {:}", index_of_configure_file_hash);

    let index_of_latest_repo_hash = hash_list
        .iter()
        .position(|r| r == hash2)
        .unwrap_or_else(|| {
            panic!(
                "The provided hash {} doesn't exist in the repository history",
                &hash2
            )
        });

    debug!("Latest repo hash is at position {:}", index_of_latest_repo_hash);

    let distance = (index_of_latest_repo_hash as i32 - index_of_configure_file_hash as i32).abs();

    Ok(distance as i32)
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
}

pub fn get_secrets_repo_status() -> Result<RepoStatus, ConfigureError> {
    let path = crate::fs::find_secrets_repo()?;

    let output = std::process::Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .arg("-b")
        .current_dir(std::fs::canonicalize(path).unwrap())
        .output()?; // Wait for it to finish and collect its output

    let status = std::str::from_utf8(&output.stdout).expect("Unable to read output data");

    Ok(parse_repo_status(status)?)
}

fn parse_repo_status(status: &str) -> Result<RepoStatus, ConfigureError> {
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

fn get_secrets_repo() -> Result<Repository, Error> {
    let path = crate::fs::find_secrets_repo().unwrap();
    Ok(Repository::open(path)?)
}
