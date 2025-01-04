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
            let cli_manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("cli/Cargo.toml");
            let manifest = cargo_toml::Manifest::from_path(cli_manifest_path).unwrap();
            let cmd = bb_imager_cli::Opt::command().display_name(manifest.package().name());

            clap_mangen::generate_to(cmd, out_dir).unwrap();
        }
        Commands::CliShellComplete { shell, out_dir } => {
            let mut cmd = bb_imager_cli::Opt::command();
            let cli_manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("cli/Cargo.toml");
            let manifest = cargo_toml::Manifest::from_path(cli_manifest_path).unwrap();

            clap_complete::generate_to(shell, &mut cmd, manifest.package().name(), out_dir)
                .unwrap();
        }
    }
}
