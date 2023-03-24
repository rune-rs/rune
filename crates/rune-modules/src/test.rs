//! `std::test` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! Note: **this has been deprecated**, all functions in this module have been
//! moved into the [`rune` crate][rune::modules].

use rune::{Module, ContextError};

/// Construct the supplemental `std::test` module.
#[deprecated = "all functions in this module have been included in the rune crate, see https://github.com/rune-rs/rune/issues/456"]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    rune::modules::test::module()
}
