//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-alloc-macros"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-alloc-macros.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-alloc-macros"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--alloc--macros-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.81+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! Macros for alloc crate of the Rune Language, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Usage
//!
//! This is part of the [Rune Language](https://rune-rs.github.io).

#![allow(clippy::manual_map)]
#![allow(clippy::enum_variant_names)]

extern crate proc_macro;

mod context;
mod try_clone;

/// Derive to implement the `TryClone` trait.
///
/// # Examples
///
/// Basic usage example:
///
/// ```
/// use rune::alloc::String;
/// use rune::alloc::clone::TryClone;
///
/// // String type implements TryClone
/// let s = String::new();
/// // ... so we can clone it
/// let copy = s.try_clone()?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// To easily implement the TryClone trait, you can also use
/// `#[derive(TryClone)]`. Example:
///
/// ```
/// use rune::alloc::clone::TryClone;
///
/// // we add the TryClone trait to Morpheus struct
/// #[derive(TryClone)]
/// struct Morpheus {
///    blue_pill: f32,
///    red_pill: i64,
/// }
///
/// let f = Morpheus { blue_pill: 0.0, red_pill: 0 };
/// // and now we can clone it!
/// let copy = f.try_clone()?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// ## Attributes
///
/// ### `try_clone(with = <path>)`
///
/// Specify a custom method when cloning a field.
///
/// ```
/// use rune::alloc::clone::TryClone;
///
/// #[derive(Debug, TryClone)]
/// #[non_exhaustive]
/// pub struct Struct {
///     #[try_clone(with = String::clone)]
///     string: String,
/// }
/// ```
///
/// ### `try_clone(try_with = <path>)`
///
/// Specify a custom fallible method when cloning a field.
///
/// ```
/// use rune::alloc::clone::TryClone;
///
/// #[derive(Debug, TryClone)]
/// #[non_exhaustive]
/// pub struct Struct {
///     #[try_clone(try_with = rune::alloc::String::try_clone)]
///     string: rune::alloc::String,
/// }
/// ```
///
/// ### `try_clone(copy)`
///
/// Specify that a field is `Copy`.
///
/// ```
/// use rune::alloc::prelude::*;
///
/// #[derive(Debug, TryClone)]
/// #[non_exhaustive]
/// pub struct Struct {
///     #[try_clone(copy)]
///     number: u32,
/// }
/// ```
#[proc_macro_derive(TryClone, attributes(try_clone))]
pub fn try_clone(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    try_clone::expand(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors<I>(errors: I) -> proc_macro2::TokenStream
where
    I: IntoIterator<Item = syn::Error>,
{
    let compile_errors = errors.into_iter().map(syn::Error::into_compile_error);
    ::quote::quote!(#(#compile_errors)*)
}
