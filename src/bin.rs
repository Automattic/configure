use log::{debug, LevelFilter};
use simplelog::CombinedLogger;
use simplelog::Config;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use structopt::StructOpt;
use structopt_flags::GetWithDefault;

#[derive(StructOpt)]
#[structopt(
    name = "configure",
    about = "A command-line utility for applying configuration secrets with strong encryption"
)]
struct Options {
    #[structopt(subcommand)]
    command: Command,

    #[structopt(flatten)]
    verbose: structopt_flags::VerboseNoDef,
}

#[derive(StructOpt)]
enum Command {
    /// Update this project's encrypted mobile secrets to the latest version
    ///
    /// This command will walk the user through updating a project's secrets by:
    ///
    /// 1. Ensuring that the mobile secrets repository has all the latest data from the server
    ///
    /// 2. Checking if the user wants to change which mobile secrets branch being used to fetch secrets
    ///
    /// 3. Prompting the user to update to the latest secrets
    ///
    /// 4. Switching the mobile secrets repo to the pinned commit hash in the `.configure` file, then copying the files specified in the `files_to_copy` hash to their specified destination, encrypting them with the format "$filename.enc".
    Update {
        /// Run the command in non-interactive mode (useful for CI or embedded contexts)
        ///
        #[structopt(short = "f", long = "force")]
        should_run_noninteractive: bool,

        #[structopt(short = "c", long = "configuration-file-path")]
        configuration_file_path: Option<String>,

        #[structopt(subcommand)]
        subcommand: Option<UpdateSubCommand>,
    },

    /// Decrypt the current mobile secrets for this project.
    ///
    Apply {
        /// Run the command in non-interactive mode (useful for CI or embedded contexts)
        ///
        #[structopt(short = "f", long = "force")]
        should_run_noninteractive: bool,

        #[structopt(short = "c", long = "configuration-file-path")]
        configuration_file_path: Option<String>,
    },

    /// Change mobile secrets settings
    ///
    /// This command will provide step-by-step help to make changes to the mobile secrets configuration.
    Init,

    /// Ensure the `.configure` file is valid
    Validate,

    /// Create a new encryption key for use with a project
    CreateKey,
}

#[derive(StructOpt)]
#[allow(clippy::enum_variant_names)] // Clippy doesn't like that these all start with `set`
enum UpdateSubCommand {
    /// Update the project name field in the .configure file for this project.
    ///
    /// Use quoted strings for multi-word entries, such as "My Project"
    SetProjectName {
        /// The new project name to write to the `project_name` field in the `.configure` file. Wrap multi-word entries in quotes, such as "My Project"
        project_name: String,
    },

    /// Update the branch name field in the .configure file for this project
    SetBranchName {
        /// The new branch name to write to the `branch` field in the `.configure` file
        branch_name: String,
    },

    /// Update the pinned commit hash field in the .configure file for this project
    SetCommitHash {
        /// The new commit hash to write to the `pinned_hash` field in the `.configure` file
        commit_hash: String,
    },
}

pub fn main() {
    let options = Options::from_args();

    match TermLogger::new(
        options.verbose.get_with_default(LevelFilter::Info),
        Config::default(),
        TerminalMode::Mixed,
    ) {
        Some(logger) => {
            CombinedLogger::init(vec![logger]).unwrap_or_default();
        }

        None => println!("Unable to initialize logging"),
    }

    debug!("libconfigure initialized");

    match Options::from_args().command {
        Command::Apply {
            should_run_noninteractive,
            configuration_file_path,
        } => configure::apply(!should_run_noninteractive, configuration_file_path),
        Command::Update {
            should_run_noninteractive,
            configuration_file_path,
            subcommand,
        } => match subcommand {
            Some(subcommand) => match subcommand {
                UpdateSubCommand::SetProjectName { project_name } => {
                    configure::update_project_name(project_name, configuration_file_path)
                }
                UpdateSubCommand::SetBranchName { branch_name } => {
                    configure::update_branch_name(branch_name, configuration_file_path)
                }
                UpdateSubCommand::SetCommitHash { commit_hash } => {
                    configure::update_pinned_hash(commit_hash, configuration_file_path)
                }
            },
            None => configure::update(!should_run_noninteractive, configuration_file_path),
        },
        Command::Init => configure::init(),
        Command::Validate => configure::validate(),
        Command::CreateKey => println!("{:}", configure::generate_encryption_key()),
    }
}
