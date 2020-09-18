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
//! <br>
//!
//! Macros for Runestick, a stack-based virtual machine for the Rust programming
//! language.
//!
//! This is part of the [Rune language].
//! [Rune Language]: https://github.com/rune-rs/rune

extern crate proc_macro;

use quote::quote;

mod any;
mod context;
mod from_value;
mod internals;
mod to_value;

/// Conversion macro for constructing proxy objects from a dynamic value.
#[proc_macro_derive(FromValue, attributes(rune))]
pub fn from_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    from_value::expand(&input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Conversion macro for constructing proxy objects from a dynamic value.
#[proc_macro_derive(ToValue, attributes(rune))]
pub fn to_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    to_value::expand(&input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Macro to mark a value as external, which will implement all the appropriate
/// traits.
///
/// This is required to support the external type as a type argument in a
/// registered function.
///
/// ## `#[rune(name = "..")]` attribute
///
/// The name of a type defaults to its identifiers, so `struct Foo {}` would be
/// given the name `"Foo"`.
///
/// This can be overrided with the `#[rune(name = "...")]` attribute:
///
/// ```rust
/// use runestick::Any;
///
/// #[derive(Any)]
/// #[rune(name = "Bar")]
/// struct Foo {
/// }
///
/// fn install() -> Result<runestick::Module, runestick::ContextError> {
///     let mut module = runestick::Module::empty();
///     module.ty::<Foo>()?;
///     Ok(module)
/// }
/// ```
#[proc_macro_derive(Any, attributes(rune))]
pub fn any(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as any::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

/// Internal macro to implement external.
#[proc_macro]
#[doc(hidden)]
pub fn __internal_impl_any(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let internal_call = syn::parse_macro_input!(input as any::InternalCall);
    internal_call
        .expand()
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
