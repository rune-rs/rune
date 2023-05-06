//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-macros"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-macros.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-macros"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--macros-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.65+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! Macros for the Rune Language, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Usage
//!
//! This is part of the [Rune Language](https://rune-rs.github.io).

extern crate proc_macro;

mod any;
mod context;
mod from_value;
mod function;
mod instrument;
mod internals;
mod macro_;
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

/// Macro used to annotate native functions which can be loaded into rune.
///
/// This macro automatically performs the following things:
/// * Rust documentation comments are captured so that it can be used in
///   generated Rune documentation.
/// * The name of arguments is captured to improve documentation generation.
/// * If an instance function is annotated this is detected (if the function
///   receives `self`). This behavior can be forced using `#[rune(instance)]` if
///   the function doesn't take `self`.
///
/// # Examples
///
/// A simple free function:
///
/// ```
/// use rune::{Module, ContextError};
///
/// /// This is a pretty neat function which is called `std::str::to_uppercase("hello")`.
/// #[rune::function]
/// fn to_uppercase(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.function_meta(to_uppercase)?;
///     Ok(m)
/// }
/// ```
///
/// A free instance function:
///
/// ```
/// use rune::{Module, ContextError};
///
/// /// This is a pretty neat function, which is called like `"hello".to_uppercase()`.
/// #[rune::function(instance)]
/// fn to_uppercase(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// /// This is a pretty neat function, which is called like `string::to_uppercase2("hello")`.
/// #[rune::function(path = string)]
/// fn to_uppercase2(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.function_meta(to_uppercase)?;
///     m.function_meta(to_uppercase2)?;
///     Ok(m)
/// }
/// ```
///
/// A regular instance function:
///
/// ```
/// use rune::{Any, Module, ContextError};
///
/// #[derive(Any)]
/// struct String {
///     inner: std::string::String
/// }
///
/// impl String {
///     /// Construct a new string wrapper.
///     #[rune::function(path = Self::new)]
///     fn new(string: &str) -> Self {
///         Self {
///             inner: string.into()
///         }
///     }
///
///     /// Construct a new string wrapper.
///     #[rune::function(path = Self::new2)]
///     fn new2(string: &str) -> Self {
///         Self {
///             inner: string.into()
///         }
///     }
///
///     /// Uppercase the string inside of the string wrapper.
///     ///
///     /// # Examples
///     ///
///     /// ```rune
///     /// let string = String::new("hello");
///     /// assert_eq!(string.to_uppercase(), "HELLO");
///     /// ```
///     #[rune::function]
///     fn to_uppercase(&self) -> std::string::String {
///         self.inner.to_uppercase()
///     }
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.ty::<String>()?;
///     m.function_meta(String::new)?;
///     m.function_meta(String::new2)?;
///     m.function_meta(String::to_uppercase)?;
///     Ok(m)
/// }
/// ```
#[proc_macro_attribute]
pub fn function(
    attrs: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attrs = syn::parse_macro_input!(attrs with crate::function::FunctionAttrs::parse);
    let function = syn::parse_macro_input!(item with crate::function::Function::parse);

    let output = match function.expand(attrs) {
        Ok(output) => output,
        Err(e) => return proc_macro::TokenStream::from(e.to_compile_error()),
    };

    output.into()
}

#[proc_macro_attribute]
pub fn macro_(
    attrs: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attrs = syn::parse_macro_input!(attrs with crate::macro_::Config::parse);
    let macro_ = syn::parse_macro_input!(item with crate::macro_::Macro::parse);

    let output = match macro_.expand(attrs) {
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
/// let foo = vm.call(["main"], ())?;
/// let foo: Foo = rune::from_value(foo)?;
///
/// assert_eq!(foo.field, 42);
/// # Ok::<_, rune::Error>(())
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
/// use rune::{ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
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
/// let value = vm.call(["main"], (Foo { field: 42 },))?;
/// let value: u64 = rune::from_value(value)?;
///
/// assert_eq!(value, 43);
/// # Ok::<_, rune::Error>(())
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
