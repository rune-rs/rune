//! The macro system of Rune.
//!
//! Macros are registered with [Module::macro_][crate::Module::macro_] and are
//! function-like items that are expanded at compile time.
//!
//! Macros take a token stream as an argument and is responsible for translating
//! it into another token stream that will be embedded into the source location
//! where the macro was invoked.
//!
//! ```
//! use rune::{T, Context, FromValue, Module, Vm};
//! use rune::ast;
//! use rune::macros::{quote, MacroContext, TokenStream};
//! use rune::parse::Parser;
//! use std::sync::Arc;
//!
//! fn concat_idents(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
//!     let mut output = String::new();
//!
//!     let mut p = Parser::from_token_stream(stream, ctx.stream_span());
//!
//!     let ident = p.parse::<ast::Ident>()?;
//!     output.push_str(ctx.resolve(ident)?);
//!
//!     while p.parse::<Option<T![,]>>()?.is_some() {
//!         if p.is_eof()? {
//!             break;
//!         }
//!
//!         let ident = p.parse::<ast::Ident>()?;
//!         output.push_str(ctx.resolve(ident)?);
//!     }
//!
//!     p.eof()?;
//!
//!     let output = ast::Ident::new(ctx, &output);
//!     Ok(quote!(#output).into_token_stream(ctx))
//! }
//!
//! # fn main() -> rune::Result<()> {
//! let mut m = Module::new();
//! m.macro_(&["concat_idents"], concat_idents)?;
//!
//! let mut context = Context::new();
//! context.install(&m)?;
//!
//! let runtime = Arc::new(context.runtime());
//!
//! let mut sources = rune::sources! {
//!     entry => {
//!         pub fn main() {
//!             let foobar = 42;
//!             concat_idents!(foo, bar)
//!         }
//!     }
//! };
//!
//! let unit = rune::prepare(&mut sources)
//!     .with_context(&context)
//!     .build()?;
//!
//! let unit = Arc::new(unit);
//!
//! let mut vm = Vm::new(runtime, unit);
//! let value = vm.call(&["main"], ())?;
//! let value = u32::from_value(value)?;
//!
//! assert_eq!(value, 42);
//! # Ok(()) }
//! ```

mod format_args;
mod macro_compiler;
mod macro_context;
mod quote_fn;
mod storage;
mod token_stream;

pub use self::format_args::FormatArgs;
pub(crate) use self::macro_compiler::MacroCompiler;
pub use self::macro_context::{IntoLit, MacroContext};
pub use self::quote_fn::{quote_fn, Quote};
pub use self::storage::{Storage, SyntheticId, SyntheticKind};
pub use self::token_stream::{ToTokens, TokenStream, TokenStreamIter};
pub use rune_macros::quote;
pub use rune_macros::ToTokens;
