//! `std::io` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! Note: **this has been deprecated**, all functions in this module have been
//! moved into the [`rune` crate][rune::modules].

use rune::{ContextError, Module};

/// Construct the supplemental `std::io` module.
#[deprecated = "all functions in this module have been included in the rune crate, see https://github.com/rune-rs/rune/issues/456"]
pub fn module(stdio: bool) -> Result<Module, ContextError> {
    rune::modules::io::module(stdio)
}
