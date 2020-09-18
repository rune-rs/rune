//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/bool/">
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
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! See each module for documentation:
//! * [http]
//! * [json]
//! * [toml]
//! * [time]
//! * [fs]
//! * [process]
//! * [signal]
//!
//! ## Features
//!
//! * `full` includes all modules.
//! * `http` for the [http module][http]
//! * `json` for the [json module][json]
//! * `toml` for the [toml module][toml]
//! * `time` for the [time module][time]
//! * `fs` for the [fs module]][fs]
//! * `process` for the [process module]][process]
//! * `signal` for the [process module]][signal]
//!
//! [http]: https://docs.rs/rune-modules/0/rune_modules/http/
//! [json]: https://docs.rs/rune-modules/0/rune_modules/json/
//! [toml]: https://docs.rs/rune-modules/0/rune_modules/toml/
//! [time]: https://docs.rs/rune-modules/0/rune_modules/time/
//! [fs]: https://docs.rs/rune-modules/0/rune_modules/fs/
//! [process]: https://docs.rs/rune-modules/0/rune_modules/process/
//! [signal]: https://docs.rs/rune-modules/0/rune_modules/signal/

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "process")]
pub mod process;

#[cfg(feature = "signal")]
pub mod signal;
