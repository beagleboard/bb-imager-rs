# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

BeagleBoard Imaging Utility (`bb-imager-rs`): a Rust workspace for creating, flashing, and managing OS images for BeagleBoard devices. It ships two front-end binaries — a GUI (`bb-imager-gui`) and a CLI (`bb-imager-cli`) — over a shared set of flasher/support libraries.

## Prerequisites

- **git-lfs is mandatory.** Assets (`*.png`, `*.ico`, `*.webp`, `*.ttf`, `*.icns`, `*.psd`) are stored via LFS. Cloning without it produces confusing "missing asset" build/runtime errors.
- Stable Rust, **edition 2024**. Builds without nightly.
- Platform build deps: `make setup-debian-deps` (Debian/Ubuntu) or `make setup-fedora-deps` (Fedora).

## Build / run / test

Run the apps directly with cargo during development:

```shell
cargo run -p bb-imager-gui
cargo run -p bb-imager-cli
```

**Prefer the Makefile for check/test over bare cargo.** Feature flags gate large amounts of code (per-board flashers, static vs. shared linking, platform SD backends), so `cargo test` / `cargo check` at the workspace root silently skip feature-gated modules and use wrong flag combinations. The Makefile targets pass the exact feature sets CI uses:

```shell
make check        # clippy (falls back to cargo check) across CLI + GUI + libs, correct features
make test         # same coverage as `make check` but runs `cargo test`
make check-cli / make check-gui
make test-cli  / make test-gui
```

`make check`/`make test` use `cargo clippy` automatically if available. Clippy and rustfmt are considered **mandatory** and non-negotiable before submitting (there is no intent to deviate from standard Rust style/lints).

To run a single test, target the crate directly (add the features that test needs):

```shell
cargo test -p bb-imager-gui <test_name>
cargo test -p bb-flasher -F sd <test_name>
```

`make help` lists all Makefile targets and tunable variables (`TARGET`, feature toggles like `PB2_MSPM0`/`ZEPTO_I2C`/`UPDATER`, packaging vars). Packaging targets (`package-<triple>`) and installs live in the Makefile too.

## Contribution conventions

- Commits must be signed off (DCO): use `git commit -s` to add `Signed-off-by:`. Reverts too (`git revert -s`). Anonymous contributions are not accepted.
- One logical change per commit/PR; write PR descriptions that state the underlying problem, then the technical solution.

## Architecture

The workspace is layered: **front-ends → `bb-flasher` façade → per-target flasher crates → support crates.**

### Front-ends
- **`bb-imager-gui`** — Iced 0.14 (Elm architecture). Not MVC; it's message-driven:
  - The top-level `enum BBImager` (in `main.rs`) *is* the app state and doubles as the screen/page state machine — each variant (`ChooseBoard`, `ChooseOs`, `ChooseDest`, `Customize`, `Review`, `Flashing`, `FlashingCancel`/`Fail`/`Success`, `AppInfo`) holds a per-screen state struct plus a shared `BBImagerCommon`.
  - `message.rs` defines `BBImagerMessage`, the single global message enum; `update` handles them and returns `iced::Task`s. `state.rs` holds per-screen state; `ui/` has one module per screen for `view`. Async work (downloads, flashing, DB) is dispatched as `Task`s, not blocking calls.
  - Local state/caching uses SQLite via `rusqlite` (`src/db/`); `persistance.rs` stores user GUI config; `bb-downloader` fetches remote images/config.
- **`bb-imager-cli`** — `clap` (derive). `cli.rs` defines the command tree (`flash` with per-target subcommands, `list-destinations`, `format`, `generate-completion`). Depends on `bb-flasher` with the `sd` feature.

### Flashing core
- **`bb-flasher`** — the common abstraction/façade the front-ends program against (`BBFlasher` trait, `LocalImage`, `sd::Flasher`, etc.). It re-exports and feature-gates the concrete flashers below; most capabilities are **off by default** and turned on per front-end via features.
- Per-target flasher crates, pulled in optionally by `bb-flasher`:
  - `bb-flasher-sd` — OS images to SD card (Linux udev / macOS authopen backends).
  - `bb-flasher-bcf` — BeagleConnect Freedom main proc (CC1352P7) and its MSP430 USB-UART bridge.
  - `bb-flasher-mspm0` / `bb-flasher-pb2-mspm0` — MSPM0 co-processor (e.g. PocketBeagle 2), UART/I2C.
  - `bb-flasher-dfu` — DFU flashing.

### Support crates
- **`bb-config`** — parse/generate the BeagleBoard `config.json`/`distros.json` (board + image catalog). See `config.json` at the repo root for the schema by example; this file normally lives on a remote server and is fetched at runtime.
- **`bb-downloader`** — async downloader with caching (JSON + file streams).
- **`bb-helper`** — shared utilities (cancellation tokens, progress-reporting readers, file streaming).
- **`bb-drivelist`** — Rust port of Balena's drivelist (enumerate destination drives).
- **`bb-bmap-parser`** — bmap parsing for sparse-image flashing.
- **`xtask`** — cargo-xtask helper crate for repo automation.

## Feature-flag notes (important for compiling correctly)

Because features cascade from front-end → `bb-flasher` → concrete flasher crates, always build/test with the intended feature set rather than defaults:

- GUI links statically by default (`static` → bundled sqlite, static lzma/hidraw, rustls). `system-deps` switches to native TLS/system libs.
- `static-hidraw` vs `shared-hidraw` selects USB HID linking (CLI defaults to static because distro hidraw is often too old).
- Board/firmware support (`bcf_cc1352p7`, `bcf_msp430`, `pb2_mspm0`, `zepto_uart`/`zepto_i2c`, `dfu`) is opt-in per binary.
- The `_check_common`/`_check_cli`/`_check_gui` recipes in the Makefile are the source of truth for which feature combinations must compile.
