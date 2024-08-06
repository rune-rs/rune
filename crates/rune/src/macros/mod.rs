//! The macro system of Rune.
//!
//! Macros are registered with [Module::macro_][crate::Module::macro_] and are
//! function-like items that are expanded at compile time.
//!
//! Macros take token streams as arguments and are responsible for translating
//! them into another token stream that will be embedded into the source location
//! where the macro was invoked.
//!
//! The attribute macros [`rune::macro_`](crate::macro_) for function macros (`some_macro!( ... )`) and
//! [`rune::attribute_macro`](crate::attribute_macro) for attribute macros (`#[some_macro ...]`).
//!
//! ```
//! use rune::{T, Context, Diagnostics, Module, Vm};
//! use rune::ast;
//! use rune::compile;
//! use rune::macros::{quote, MacroContext, TokenStream, ToTokens};
//! use rune::parse::Parser;
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use rune::alloc::String;
//!
//! use std::sync::Arc;
//!
//! #[rune::macro_]
//! fn concat_idents(cx: &mut MacroContext<'_, '_, '_>, input: &TokenStream) -> compile::Result<TokenStream> {
//!     let mut output = String::new();
//!
//!     let mut p = Parser::from_token_stream(input, cx.input_span());
//!
//!     let ident = p.parse::<ast::Ident>()?;
//!     output.try_push_str(cx.resolve(ident)?)?;
//!
//!     while p.parse::<Option<T![,]>>()?.is_some() {
//!         if p.is_eof()? {
//!             break;
//!         }
//!
//!         let ident = p.parse::<ast::Ident>()?;
//!         output.try_push_str(cx.resolve(ident)?)?;
//!     }
//!
//!     p.eof()?;
//!
//!     let output = cx.ident(&output)?;
//!     Ok(quote!(#output).into_token_stream(cx)?)
//! }
//!
//! #[rune::attribute_macro]
//! fn rename(cx: &mut MacroContext<'_, '_, '_>, input: &TokenStream, item: &TokenStream) -> compile::Result<TokenStream> {
//!     let mut parser = Parser::from_token_stream(item, cx.macro_span());
//!     let mut fun: ast::ItemFn = parser.parse_all()?;
//!
//!     let mut parser = Parser::from_token_stream(input, cx.input_span());
//!     fun.name = parser.parse_all::<ast::EqValue<_>>()?.value;
//!
//!     let mut tokens = TokenStream::new();
//!     fun.to_tokens(cx, &mut tokens);
//!     Ok(tokens)
//! }
//!
//! let mut m = Module::new();
//! m.macro_meta(concat_idents)?;
//! m.macro_meta(rename)?;
//!
//! let mut context = Context::new();
//! context.install(m)?;
//!
//! let runtime = Arc::new(context.runtime()?);
//!
//! let mut sources = rune::sources! {
//!     entry => {
//!         #[rename = foobar]
//!         fn renamed() {
//!             42
//!         }
//!
//!         pub fn main() {
//!             let foobar = foobar();
//!             concat_idents!(foo, bar)
//!         }
//!     }
//! };
//!
//! let mut diagnostics = Diagnostics::new();
//!
//! let result = rune::prepare(&mut sources)
//!     .with_context(&context)
//!     .with_diagnostics(&mut diagnostics)
//!     .build();
//!
//! if !diagnostics.is_empty() {
//!     let mut writer = StandardStream::stderr(ColorChoice::Always);
//!     diagnostics.emit(&mut writer, &sources)?;
//! }
//!
//! let unit = result?;
//! let unit = Arc::new(unit);
//!
//! let mut vm = Vm::new(runtime, unit);
//! let value = vm.call(["main"], ())?;
//! let value: u32 = rune::from_value(value)?;
//!
//! assert_eq!(value, 42);
//! # Ok::<_, rune::support::Error>(())
//! ```

mod format_args;
mod into_lit;
mod macro_compiler;
mod macro_context;
mod quote_fn;
mod storage;
mod token_stream;

pub use self::format_args::FormatArgs;
pub use self::into_lit::IntoLit;
pub(crate) use self::macro_compiler::MacroCompiler;
#[cfg(feature = "std")]
pub use self::macro_context::test;
pub use self::macro_context::MacroContext;
pub use self::quote_fn::{quote_fn, Quote};
pub(crate) use self::storage::Storage;
pub use self::storage::{SyntheticId, SyntheticKind};
pub use self::token_stream::{ToTokens, TokenStream, TokenStreamIter};

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
pub use rune_macros::quote;

/// Helper derive to implement [`ToTokens`].
pub use rune_macros::ToTokens;
