use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

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
}

fn main() {
    let opts = Opt::parse();

    match opts.command {
        Commands::CliMan { out_dir } => {
            let cmd = bb_imager_cli::Opt::command();
            clap_mangen::generate_to(cmd, out_dir).unwrap()
        }
    }
}
