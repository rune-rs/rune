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
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/CI/badge.svg">
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
//! This is part of the [Rune Language](https://rune-rs.github.io).
extern crate proc_macro;

mod any;
mod context;
mod from_value;
mod instrument;
mod internals;
mod opaque;
mod option_spanned;
mod parse;
mod quote;
mod spanned;
mod to_tokens;
mod to_value;

/// Macro helper function for quoting the token stream as macro output.
///
/// Is capable of quoting everything in Rune, except for the following:
/// * Labels, which must be created using `Label::new`.
/// * Dynamic quoted strings and other literals, which must be created using
///   `Lit::new`.
///
/// ```
/// use rune::macros::quote;
///
/// quote!(hello self);
/// ```
///
/// # Interpolating values
///
/// Values are interpolated with `#value`, or `#(value + 1)` for expressions.
///
/// # Iterators
///
/// Anything that can be used as an iterator can be iterated over with
/// `#(iter)*`. A token can also be used to join inbetween each iteration, like
/// `#(iter),*`.
#[proc_macro]
pub fn quote(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let parser = crate::quote::Quote::new();

    let output = match parser.parse(input) {
        Ok(output) => output,
        Err(e) => return proc_macro::TokenStream::from(e.to_compile_error()),
    };

    output.into()
}

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

/// Helper derive to implement `Opaque`.
#[proc_macro_derive(Opaque, attributes(rune))]
#[doc(hidden)]
pub fn opaque(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = syn::parse_macro_input!(input as opaque::Derive);
    derive.expand().unwrap_or_else(to_compile_errors).into()
}

/// Derive macro for the `FromValue` trait for converting types from the dynamic
/// `Value` container.
///
/// # Examples
///
/// ```
/// use rune::{FromValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(FromValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// # fn main() -> rune::Result<()> {
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main() {
///             #{field: 42}
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(&["main"], ())?;
/// let foo = Foo::from_value(foo)?;
///
/// assert_eq!(foo.field, 42);
/// # Ok(()) }
/// ```
#[proc_macro_derive(FromValue, attributes(rune))]
pub fn from_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    from_value::expand(&input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Derive macro for the `FromValue` trait for converting types into the dynamic
/// `Value` container.
///
/// # Examples
///
/// ```
/// use rune::{FromValue, ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// # fn main() -> rune::Result<()> {
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main(foo) {
///             foo.field + 1
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(&["main"], (Foo { field: 42 },))?;
/// let foo = u64::from_value(foo)?;
///
/// assert_eq!(foo, 43);
/// # Ok(()) }
/// ```
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
/// ```
/// use rune::Any;
///
/// #[derive(Any)]
/// #[rune(name = "Bar")]
/// struct Foo {
/// }
///
/// fn install() -> Result<rune::Module, rune::ContextError> {
///     let mut module = rune::Module::new();
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

/// Internal macro to instrument a function which is threading AST.
#[proc_macro_attribute]
#[doc(hidden)]
pub fn __instrument_ast(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let internal_call = syn::parse_macro_input!(item as instrument::Expander);
    internal_call
        .expand()
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.into_iter().map(syn::Error::into_compile_error);
    ::quote::quote!(#(#compile_errors)*)
}
