use std::path::PathBuf;

use clap::CommandFactory;

// Pull in the CLI definition directly instead of depending on the crate itself, so the generated
// man pages and shell completions always reflect the exact command tree (including feature-gated
// subcommands) that this build produces.
#[path = "src/cli.rs"]
mod cli;

fn main() {
    // The generated artifacts only depend on the command definition.
    println!("cargo::rerun-if-changed=src/cli.rs");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR set by cargo"));
    let name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME set by cargo");

    let man_dir = out_dir.join("man");
    std::fs::create_dir_all(&man_dir).unwrap();
    let mut cmd = cli::Opt::command().display_name(&name);
    clap_mangen::generate_to(cmd.clone(), &man_dir).unwrap();

    let comp_dir = out_dir.join("shell-comp");
    std::fs::create_dir_all(&comp_dir).unwrap();
    for shell in [clap_complete::Shell::Bash, clap_complete::Shell::Zsh] {
        clap_complete::generate_to(shell, &mut cmd, &name, &comp_dir).unwrap();
    }
}
