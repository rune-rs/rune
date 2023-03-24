//! Abstract syntax trees for the Rune language.
//!
//! These are primarily made available for use in macros, where the input to the
//! macro needs to be parsed so that it can be processed.
//!
//! Below we define a macro capable of taking identifiers like `hello`, and
//! turning them into literal strings like `"hello"`.
//!
//! ```
//! use rune::{Context, FromValue, Module, Vm};
//! use rune::ast;
//! use rune::macros::{quote, MacroContext, TokenStream};
//! use rune::parse::Parser;
//! use std::sync::Arc;
//!
//! fn ident_to_string(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
//!     let mut p = Parser::from_token_stream(stream, ctx.stream_span());
//!     let ident = p.parse_all::<ast::Ident>()?;
//!     let ident = ctx.resolve(ident)?.to_owned();
//!     let string = ctx.lit(&ident);
//!     Ok(quote!(#string).into_token_stream(ctx))
//! }
//!
//! # fn main() -> rune::Result<()> {
//! let mut m = Module::new();
//! m.macro_(["ident_to_string"], ident_to_string)?;
//!
//! let mut context = Context::new();
//! context.install(m)?;
//!
//! let runtime = Arc::new(context.runtime());
//!
//! let mut sources = rune::sources! {
//!     entry => {
//!         pub fn main() {
//!             ident_to_string!(hello)
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
//! let value = vm.call(["main"], ())?;
//! let value = String::from_value(value)?;
//!
//! assert_eq!(value, "hello");
//! # Ok(()) }
//! ```

use crate::macros::{MacroContext, ToTokens, TokenStream};
use crate::parse::{Parse, ParseError, Parser, Peek};

#[macro_use]
/// Generated modules.
mod generated;
pub use self::generated::*;

macro_rules! expr_parse {
    ($ty:ident, $local:ty, $expected:literal) => {
        impl $crate::parse::Parse for $local {
            fn parse(p: &mut $crate::parse::Parser<'_>) -> Result<Self, $crate::parse::ParseError> {
                let t = p.tok_at(0)?;

                match $crate::ast::Expr::parse(p)? {
                    $crate::ast::Expr::$ty(expr) => Ok(expr),
                    _ => Err($crate::parse::ParseError::expected(t, $expected)),
                }
            }
        }
    };
}

macro_rules! item_parse {
    ($ty:ident, $local:ty, $expected:literal) => {
        impl $crate::parse::Parse for $local {
            fn parse(p: &mut $crate::parse::Parser<'_>) -> Result<Self, $crate::parse::ParseError> {
                let t = p.tok_at(0)?;

                match $crate::ast::Item::parse(p)? {
                    $crate::ast::Item::$ty(item) => Ok(item),
                    _ => Err($crate::parse::ParseError::expected(t, $expected)),
                }
            }
        }
    };
}

mod attribute;
mod block;
mod condition;
mod expr;
mod expr_assign;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_continue;
mod expr_empty;
mod expr_field_access;
mod expr_for;
mod expr_group;
mod expr_if;
mod expr_index;
mod expr_let;
mod expr_lit;
mod expr_loop;
mod expr_match;
mod expr_object;
mod expr_range;
mod expr_return;
mod expr_select;
mod expr_try;
mod expr_tuple;
mod expr_unary;
mod expr_vec;
mod expr_while;
mod expr_yield;
mod file;
mod fn_arg;
mod grouped;
mod ident;
mod item;
mod item_const;
mod item_enum;
mod item_fn;
mod item_impl;
mod item_mod;
mod item_struct;
mod item_use;
mod label;
mod lit;
mod lit_bool;
mod lit_byte;
mod lit_byte_str;
mod lit_char;
mod lit_number;
mod lit_str;
mod local;
mod macro_call;
mod pat;
mod path;
mod prelude;
mod span;
mod spanned;
mod spanned_error;
mod stmt;
mod token;
pub(super) mod utils;
mod vis;

pub use self::attribute::Attribute;
pub use self::block::Block;
pub use self::condition::Condition;
pub use self::expr::Expr;
pub use self::expr_assign::ExprAssign;
pub use self::expr_await::ExprAwait;
pub use self::expr_binary::{BinOp, ExprBinary};
pub use self::expr_block::ExprBlock;
pub use self::expr_break::{ExprBreak, ExprBreakValue};
pub use self::expr_call::ExprCall;
pub use self::expr_closure::{ExprClosure, ExprClosureArgs};
pub use self::expr_continue::ExprContinue;
pub use self::expr_empty::ExprEmpty;
pub use self::expr_field_access::{ExprField, ExprFieldAccess};
pub use self::expr_for::ExprFor;
pub use self::expr_group::ExprGroup;
pub use self::expr_if::{ExprElse, ExprElseIf, ExprIf};
pub use self::expr_index::ExprIndex;
pub use self::expr_let::ExprLet;
pub use self::expr_lit::ExprLit;
pub use self::expr_loop::ExprLoop;
pub use self::expr_match::{ExprMatch, ExprMatchBranch};
pub use self::expr_object::{ExprObject, FieldAssign, ObjectIdent, ObjectKey};
pub use self::expr_range::{ExprRange, ExprRangeLimits};
pub use self::expr_return::ExprReturn;
pub use self::expr_select::{ExprSelect, ExprSelectBranch};
pub use self::expr_try::ExprTry;
pub use self::expr_tuple::ExprTuple;
pub use self::expr_unary::{ExprUnary, UnOp};
pub use self::expr_vec::ExprVec;
pub use self::expr_while::ExprWhile;
pub use self::expr_yield::ExprYield;
pub use self::file::{File, Shebang};
pub use self::fn_arg::FnArg;
pub use self::grouped::{AngleBracketed, Braced, Bracketed, Parenthesized};
pub use self::ident::Ident;
pub use self::item::Item;
pub use self::item_const::ItemConst;
pub use self::item_enum::{ItemEnum, ItemVariant, ItemVariantBody};
pub use self::item_fn::ItemFn;
pub use self::item_impl::ItemImpl;
pub use self::item_mod::{ItemInlineBody, ItemMod, ItemModBody};
pub use self::item_struct::{Field, ItemStruct, ItemStructBody};
pub use self::item_use::{ItemUse, ItemUsePath, ItemUseSegment};
pub use self::label::Label;
pub use self::lit::Lit;
pub use self::lit_bool::LitBool;
pub use self::lit_byte::LitByte;
pub use self::lit_byte_str::LitByteStr;
pub use self::lit_char::LitChar;
pub use self::lit_number::LitNumber;
pub use self::lit_str::LitStr;
pub use self::local::Local;
pub use self::macro_call::MacroCall;
pub use self::pat::{Pat, PatBinding, PatLit, PatObject, PatPath, PatTuple, PatVec};
pub use self::path::{Path, PathKind, PathSegment, PathSegmentExpr};
pub use self::span::{ByteIndex, Span};
pub use self::spanned::{OptionSpanned, Spanned};
pub use self::spanned_error::SpannedError;
pub(crate) use self::spanned_error::WithSpan;
pub use self::stmt::{ItemOrExpr, Stmt, StmtSemi, StmtSortKey};
pub use self::token::{
    BuiltIn, CopySource, Delimiter, LitSource, Number, NumberBase, NumberSource, NumberText,
    StrSource, StrText, Token,
};
pub use self::vis::Visibility;

macro_rules! decl_tokens {
    ($(($parser:ident, $name:expr, $doc:expr, $($kind:tt)*),)*) => {
        $(
            #[doc = $doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $parser {
                /// Associated token.
                pub span: Span,
            }

            impl Spanned for $parser {
                fn span(&self) -> Span {
                    self.span
                }
            }

            impl Parse for $parser {
                fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
                    let t = parser.next()?;

                    match t.kind {
                        $($kind)* => Ok(Self { span: t.span }),
                        _ => Err(ParseError::expected(t, $name)),
                    }
                }
            }

            impl Peek for $parser {
                fn peek(p: &mut $crate::parse::Peeker<'_>) -> bool {
                    matches!(p.nth(0), $($kind)*)
                }
            }

            impl ToTokens for $parser {
                fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
                    stream.push(Token { span: self.span, kind: $($kind)* });
                }
            }
        )*
    }
}

decl_tokens! {
    (CloseBrace, "a closing brace `}`", "closing brace", Kind::Close(Delimiter::Brace)),
    (CloseBracket, "a closing bracket `]`", "closing bracket", Kind::Close(Delimiter::Bracket)),
    (CloseParen, "a closing parenthesis `)`", "closing parenthesis", Kind::Close(Delimiter::Parenthesis)),
    (CloseEmpty, "an empty closing marker", "closing marker", Kind::Close(Delimiter::Empty)),
    (OpenBrace, "an opening brace `{`", "opening brace", Kind::Open(Delimiter::Brace)),
    (OpenBracket, "an open bracket `[`", "opening bracket", Kind::Open(Delimiter::Bracket)),
    (OpenParen, "an opening parenthesis `(`", "opening parenthesis", Kind::Open(Delimiter::Parenthesis)),
    (OpenEmpty, "an empty opening marker", "opening marker", Kind::Open(Delimiter::Empty)),
}

/// The composite `is not` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToTokens, Spanned)]
#[non_exhaustive]
pub struct IsNot {
    /// The `is` token.
    pub is: Is,
    /// The `not` token.
    pub not: Not,
}
