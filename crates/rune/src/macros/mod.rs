//! The macro system of Rune.

mod functions;
mod macro_compiler;
mod macro_context;
mod quote;
mod storage;
mod token_stream;

pub use self::functions::{eval, resolve, stringify, to_tokens};
pub(crate) use self::macro_context::EvaluationContext;
pub use self::macro_context::{with_context, IntoLit, MacroContext};
pub use self::storage::Storage;
pub use self::token_stream::{ToTokens, TokenStream, TokenStreamIter};

pub(crate) use self::macro_compiler::MacroCompiler;
pub(crate) use self::macro_context::current_context;
