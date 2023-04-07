//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/book/">
//!     <b>Read the book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/CI/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! <br>
//!
//! The language server for the [Rune language].
//!
//! [Rune Language]: https://rune-rs.github.io

use anyhow::{bail, Result};
use rune::Options;
use std::env;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

fn setup_logging() -> Result<Option<WorkerGuard>> {
    let mut guard = None;

    let env_filter = EnvFilter::from_env("RUNE_LOG");

    // Set environment variable to get the language server to trace log to the
    // given file.
    if let Some(log_path) = std::env::var_os("RUNE_LOG_FILE") {
        let log_path = PathBuf::from(log_path);

        if let (Some(d), Some(name)) = (log_path.parent(), log_path.file_name()) {
            let file_appender = tracing_appender::rolling::never(d, name);
            let (non_blocking, g) = tracing_appender::non_blocking(file_appender);

            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(non_blocking)
                .init();

            guard = Some(g);
        }
    }

    Ok(guard)
}

fn main() -> Result<()> {
    let _guard = setup_logging()?;

    let mut it = env::args();
    it.next();

    #[allow(clippy::never_loop)]
    for arg in it {
        match arg.as_str() {
            "--version" => {
                println!("Rune language server {}", VERSION);
                return Ok(());
            }
            other => {
                bail!("Unsupported option: {}", other);
            }
        }
    }

    let mut context = rune_modules::default_context()?;
    context.install(rune_modules::experiments::module(true)?)?;

    let mut options = Options::default();
    options.macros(true);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = runtime.block_on(rune::languageserver::run(context, options));

    match result {
        Ok(()) => {
            tracing::info!("Server shutting down");
        }
        Err(error) => {
            tracing::error!("Server errored: {error}");
        }
    }

    Ok(())
}
