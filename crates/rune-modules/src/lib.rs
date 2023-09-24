//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-modules"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-modules.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-modules"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--modules-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
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
//! Native modules for Rune, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Usage
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
//! <br>
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

#[cfg(feature = "experiments")]
pub mod experiments;

macro_rules! modules {
    ($({$ident:ident, $name:literal $(, $module:ident)*}),* $(,)?) => {
        $(
            #[cfg(feature = $name)]
            pub mod $ident;
        )*

        /// Construct a a default rune context with all enabled modules provided
        /// based on the [default rune
        /// context](rune::Context::with_default_modules).
        pub fn with_config(stdio: bool) -> Result<rune::Context, rune::ContextError> {
            #[allow(unused_mut)]
            let mut context = rune::Context::with_config(stdio)?;

            $(
                #[allow(deprecated)]
                #[cfg(feature = $name)]
                {
                    context.install(self::$ident::module(stdio)?)?;
                    $(context.install(self::$ident::$module::module(stdio)?)?;)*
                }
            )*

            Ok(context)
        }

        /// Construct a a default context rune context with default config.
        pub fn default_context() -> Result<rune::Context, rune::ContextError> {
            with_config(true)
        }
    }
}

modules! {
    {core, "core"},
    {fmt, "fmt"},
    {fs, "fs"},
    {http, "http"},
    {io, "io"},
    {json, "json"},
    {macros, "macros"},
    {process, "process"},
    {rand, "rand"},
    {signal, "signal"},
    {test, "test"},
    {time, "time"},
    {toml, "toml", ser, de},
}
