use clap::Parser;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    let opt = bb_imager_cli::cli::Opt::parse();

    if opt.verbose {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .expect("Failed to register tracing_subscriber");
    }

    bb_imager_cli::run(opt);
}
