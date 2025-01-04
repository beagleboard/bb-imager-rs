use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

#[path = "../../cli/src/cli.rs"]
// Allow using CLI stuff without pulling bb-imager-cli and bb-imager as dependencies
mod bb_imager_cli;

#[derive(Parser)]
struct Opt {
    #[command(subcommand)]
    /// Specifies the subcommand to execute.
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Manpage for CLI
    CliMan {
        /// Directory to save manpages
        out_dir: PathBuf,
    },
    /// Generate Shell Completion for CLI
    CliShellComplete {
        /// Target shell
        shell: clap_complete::Shell,
        /// Directory to save manpages
        out_dir: PathBuf,
    },
}

fn main() {
    let opts = Opt::parse();

    match opts.command {
        Commands::CliMan { out_dir } => {
            let cmd = bb_imager_cli::Opt::command();
            clap_mangen::generate_to(cmd, out_dir).unwrap();
        }
        Commands::CliShellComplete { shell, out_dir } => {
            let mut cmd = bb_imager_cli::Opt::command();
            const CLI_BIN_NAME: &str = "bb-imager-cli";

            clap_complete::generate_to(shell, &mut cmd, CLI_BIN_NAME, out_dir).unwrap();
        }
    }
}
