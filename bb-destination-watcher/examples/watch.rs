//! Prints a line every time the platform reports a storage change.
//!
//! Useful for checking a backend by hand:
//!
//! ```text
//! cargo run -p bb-destination-watcher --example watch
//! ```
//!
//! then plug in a USB stick or an SD card. On Linux a loop device works too:
//! `udisksctl loop-setup -f some.img`.

use std::time::Instant;

use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    // `RUST_LOG=debug` prints every signal the backend saw, including the ones
    // filtered out.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let mut watcher = match bb_destination_watcher::Watcher::new().await {
        Ok(w) => w,
        Err(e) => {
            eprintln!("no watcher available: {e}");
            std::process::exit(1);
        }
    };

    // stderr: unbuffered, so output survives redirection and Ctrl-C.
    eprintln!("watching for storage changes; Ctrl-C to stop");

    let start = Instant::now();
    while watcher.next().await.is_some() {
        eprintln!("[{:>8.3}s] storage changed", start.elapsed().as_secs_f64());
    }

    eprintln!("watcher ended");
}
