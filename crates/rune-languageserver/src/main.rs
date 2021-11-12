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
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
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

fn setup_logging() -> Result<()> {
    // Set environment variable to get the language server to trace log to the
    // given file.
    if let Some(log_path) = std::env::var_os("RUNE_TRACE_LOG_FILE") {
        use log::LevelFilter;
        use log4rs::append::file::FileAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::default()))
            .build(log_path)?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    setup_logging()?;

    let mut it = env::args();
    it.next();

    #[allow(clippy::never_loop)]
    for arg in it {
        match arg.as_str() {
            "--version" => {
                println!("Rune language server {}", rune_languageserver::VERSION);
                return Ok(());
            }
            other => {
                bail!("Unsupported option: {}", other);
            }
        }
    }

    let mut context = rune_modules::default_context()?;
    context.install(&rune_modules::experiments::module(true)?)?;

    let mut options = Options::default();
    options.macros(true);

    rune_languageserver::run(context, options)
}
