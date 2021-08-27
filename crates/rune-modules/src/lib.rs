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
//! Native modules for the runestick virtual machine.
//!
//! These are modules that can be used with the [Rune language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! See each module for documentation:
//! * [core]
//! * [experiments]
//! * [fmt]
//! * [fs]
//! * [http]
//! * [io]
//! * [json]
//! * [macros]
//! * [process]
//! * [rand]
//! * [signal]
//! * [test]
//! * [time]
//! * [toml]
//!
//! ## Features
//!
//! * `core` for the [core module][toml]
//! * `experiments` for the [experiments module][experiments]
//! * `fmt` for the [fmt module][fmt]
//! * `fs` for the [fs module][fs]
//! * `full` includes all modules.
//! * `http` for the [http module][http]
//! * `io` for the [io module][io]
//! * `json` for the [json module][json]
//! * `macros` for the [macros module][macros]
//! * `process` for the [process module][process]
//! * `rand` for the [rand module][rand]
//! * `signal` for the [signal module][signal]
//! * `test` for the [test module][test]
//! * `time` for the [time module][time]
//! * `toml` for the [toml module][toml]
//!
//! [core]: https://docs.rs/rune-modules/0/rune_modules/core/
//! [experiments]: https://docs.rs/rune-modules/0/rune_modules/experiments/
//! [fmt]: https://docs.rs/rune-modules/0/rune_modules/fmt/
//! [fs]: https://docs.rs/rune-modules/0/rune_modules/fs/
//! [http]: https://docs.rs/rune-modules/0/rune_modules/http/
//! [io]: https://docs.rs/rune-modules/0/rune_modules/io/
//! [json]: https://docs.rs/rune-modules/0/rune_modules/json/
//! [macros]: https://docs.rs/rune-modules/0/rune_modules/macros/
//! [process]: https://docs.rs/rune-modules/0/rune_modules/process/
//! [rand]: https://docs.rs/rune-modules/0/rune_modules/rand/
//! [signal]: https://docs.rs/rune-modules/0/rune_modules/signal/
//! [test]: https://docs.rs/rune-modules/0/rune_modules/test/
//! [time]: https://docs.rs/rune-modules/0/rune_modules/time/
//! [toml]: https://docs.rs/rune-modules/0/rune_modules/toml/

// Note: The above links to docs.rs are needed because cargo-readme does not
// support intra-doc links (yet):
// https://github.com/livioribeiro/cargo-readme/issues/55
#![allow(clippy::needless_borrow, clippy::useless_format)]
#[cfg(feature = "experiments")]
pub mod experiments;

macro_rules! modules {
    ($($ident:ident, $name:literal),* $(,)?) => {
        $(
            #[cfg(feature = $name)]
            pub mod $ident;
        )*

        /// Construct a a default context runestick context with all enabled
        /// modules provided based on the [default runestick
        /// context](runestick::Context::with_default_modules).
        pub fn with_config(stdio: bool) -> Result<runestick::Context, runestick::ContextError> {
            #[allow(unused_mut)]
            let mut context = runestick::Context::with_config(stdio)?;

            $(
                #[cfg(feature = $name)]
                {
                    context.install(&self::$ident::module(stdio)?)?;
                }
            )*

            Ok(context)
        }

        /// Construct a a default context runestick context with default config.
        pub fn default_context() -> Result<runestick::Context, runestick::ContextError> {
            with_config(true)
        }
    }
}

modules! {
    core, "core",
    fmt, "fmt",
    fs, "fs",
    http, "http",
    io, "io",
    json, "json",
    macros, "macros",
    process, "process",
    rand, "rand",
    signal, "signal",
    test, "test",
    time, "time",
    toml, "toml",
}
