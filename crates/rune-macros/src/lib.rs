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
//! Macros for Rune.
//!
//! This is part of the [Rune language].
//! [Rune Language]: https://rune-rs.github.io

extern crate proc_macro;

use quote::quote;

mod context;
mod internals;
mod option_spanned;
mod parse;
mod spanned;
mod to_tokens;

/// Helper derive to implement `ToTokens`.
#[proc_macro_derive(ToTokens, attributes(rune))]
#[doc(hidden)]
pub fn to_tokens(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as to_tokens::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

/// Helper derive to implement `Parse`.
#[proc_macro_derive(Parse, attributes(rune))]
#[doc(hidden)]
pub fn parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as parse::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

/// Helper derive to implement `Spanned`.
#[proc_macro_derive(Spanned, attributes(rune))]
#[doc(hidden)]
pub fn spanned(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as spanned::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

/// Helper derive to implement `OptionSpanned`.
#[proc_macro_derive(OptionSpanned, attributes(rune))]
#[doc(hidden)]
pub fn option_spanned(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as option_spanned::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
