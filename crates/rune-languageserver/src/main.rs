//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-languageserver"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-languageserver.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-languageserver"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--languageserver-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.70+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! A language server for the Rune Language, an embeddable dynamic programming language for Rust.
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
