//! AST for the Rune language.

use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek};
use runestick::Span;

/// A helper macro to implement [`Peek`].
macro_rules! peek {
    ($expr:expr) => {
        peek!($expr, false)
    };

    ($expr:expr, $default:expr) => {
        match $expr {
            Some(value) => value,
            None => return $default,
        }
    };
}

macro_rules! expr_parse {
    ($ty:ident, $expected:literal) => {
        impl crate::Parse for $ty {
            fn parse(parser: &mut crate::Parser<'_>) -> Result<Self, crate::ParseError> {
                let t = parser.token_peek_eof()?;
                let expr = crate::ast::Expr::parse(parser)?;

                match expr {
                    crate::ast::Expr::$ty(expr) => Ok(expr),
                    _ => Err(crate::ParseError::expected(t, $expected)),
                }
            }
        }
    };
}

macro_rules! item_parse {
    ($ty:ident, $expected:literal) => {
        impl crate::Parse for $ty {
            fn parse(parser: &mut crate::Parser<'_>) -> Result<Self, crate::ParseError> {
                let t = parser.token_peek_eof()?;
                let expr = crate::ast::Item::parse(parser)?;

                match expr {
                    crate::ast::Item::$ty(item) => Ok(item),
                    _ => Err(crate::ParseError::expected(t, $expected)),
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
mod expr_field_access;
mod expr_for;
mod expr_group;
mod expr_if;
mod expr_index;
mod expr_let;
mod expr_lit;
mod expr_loop;
mod expr_match;
mod expr_return;
mod expr_select;
mod expr_try;
mod expr_unary;
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
mod lit_object;
mod lit_str;
mod lit_template;
mod lit_tuple;
mod lit_unit;
mod lit_vec;
mod local;
mod macro_call;
mod pat;
mod path;
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
pub use self::expr_closure::ExprClosure;
pub use self::expr_field_access::{ExprField, ExprFieldAccess};
pub use self::expr_for::ExprFor;
pub use self::expr_group::ExprGroup;
pub use self::expr_if::{ExprElse, ExprElseIf, ExprIf};
pub use self::expr_index::ExprIndex;
pub use self::expr_let::ExprLet;
pub use self::expr_lit::ExprLit;
pub use self::expr_loop::ExprLoop;
pub use self::expr_match::{ExprMatch, ExprMatchBranch};
pub use self::expr_return::ExprReturn;
pub use self::expr_select::{ExprSelect, ExprSelectBranch};
pub use self::expr_try::ExprTry;
pub use self::expr_unary::{ExprUnary, UnOp};
pub use self::expr_while::ExprWhile;
pub use self::expr_yield::ExprYield;
pub use self::file::File;
pub use self::fn_arg::FnArg;
pub use self::grouped::{Braced, Bracketed, Parenthesized};
pub use self::ident::Ident;
pub use self::item::Item;
pub use self::item_const::ItemConst;
pub use self::item_enum::{ItemEnum, ItemVariant, ItemVariantBody};
pub use self::item_fn::ItemFn;
pub use self::item_impl::ItemImpl;
pub use self::item_mod::{ItemMod, ItemModBody};
pub use self::item_struct::{Field, ItemStruct, ItemStructBody};
pub use self::item_use::{ItemUse, ItemUsePath, ItemUseSegment};
pub use self::label::Label;
pub use self::lit::Lit;
pub use self::lit_bool::LitBool;
pub use self::lit_byte::LitByte;
pub use self::lit_byte_str::LitByteStr;
pub use self::lit_char::LitChar;
pub use self::lit_number::LitNumber;
pub use self::lit_object::{
    AnonymousLitObject, LitObject, LitObjectFieldAssign, LitObjectIdent, LitObjectKey,
};
pub use self::lit_str::LitStr;
pub use self::lit_template::{LitTemplate, Template, TemplateComponent};
pub use self::lit_tuple::LitTuple;
pub use self::lit_unit::LitUnit;
pub use self::lit_vec::LitVec;
pub use self::local::Local;
pub use self::macro_call::MacroCall;
pub use self::pat::{Pat, PatBinding, PatLit, PatObject, PatPath, PatTuple, PatVec};
pub use self::path::{Path, PathKind, PathSegment};
pub use self::stmt::Stmt;
pub use self::token::{
    CopySource, Delimiter, Kind, LitByteStrSource, LitByteStrSourceText, LitStrSource,
    LitStrSourceText, Number, NumberBase, NumberSource, NumberSourceText, StringSource, Token,
};
pub use self::vis::Visibility;

macro_rules! decl_tokens {
    ($(($parser:ident, $doc:expr, $($kind:tt)*),)*) => {
        $(
            #[doc = $doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $parser {
                /// Associated token.
                pub token: Token,
            }

            impl crate::Spanned for $parser {
                fn span(&self) -> Span {
                    self.token.span()
                }
            }

            impl Parse for $parser {
                fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
                    let token = parser.token_next()?;

                    match token.kind {
                        $($kind)* => Ok(Self {
                            token,
                        }),
                        _ => Err(ParseError::new(token, ParseErrorKind::TokenMismatch {
                            expected: $($kind)*,
                            actual: token.kind,
                        })),
                    }
                }
            }

            impl Peek for $parser {
                fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
                    match p1 {
                        Some(p1) => matches!(p1.kind, $($kind)*),
                        _ => false,
                    }
                }
            }

            impl crate::ToTokens for $parser {
                fn to_tokens(&self, _: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
                    stream.push(self.token);
                }
            }
        )*
    }
}

decl_tokens! {
    (And, "And `&&` operator.", Kind::AmpAmp),
    (As, "The `as` keyword.", Kind::As),
    (Async, "The `async` keyword.", Kind::Async),
    (Await, "The `await` keyword.", Kind::Await),
    (Bang, "The `!` operator.", Kind::Bang),
    (Break, "The `break` keyword.", Kind::Break),
    (CloseBrace, "An closing brace `}`.", Kind::Close(Delimiter::Brace)),
    (CloseBracket, "An open bracket `]`.", Kind::Close(Delimiter::Bracket)),
    (CloseParen, "An closing parenthesis `)`.", Kind::Close(Delimiter::Parenthesis)),
    (Colon, "A colon `:`.", Kind::Colon),
    (Comma, "A comma `,`.", Kind::Comma),
    (Const, "The `const` keyword.", Kind::Const),
    (Crate, "The `crate` keyword.", Kind::Crate),
    (Default, "The `default` keyword.", Kind::Default),
    (Dot, "A dot `.`.", Kind::Dot),
    (DotDot, "Two dots `..`.", Kind::DotDot),
    (Else, "The `else` keyword.", Kind::Else),
    (Enum, "The `enum` keyword.", Kind::Enum),
    (Eq, "An equals sign `=`.", Kind::Eq),
    (Extern, "The `extern` keyword.", Kind::Extern),
    (Fn, "The `fn` keyword.", Kind::Fn),
    (For, "The `for` keyword.", Kind::For),
    (Hash, "The hash `#`.", Kind::Pound),
    (If, "The `if` keyword.", Kind::If),
    (Impl, "The `impl` keyword", Kind::Impl),
    (In, "The `in` keyword.", Kind::In),
    (Is, "The `is` keyword.", Kind::Is),
    (Let, "The `let` keyword.", Kind::Let),
    (Loop, "The `loop` keyword.", Kind::Loop),
    (Match, "The `match` keyword.", Kind::Match),
    (Mod, "The `mod` keyword.", Kind::Mod),
    (Mul, "Multiply `*` operator.", Kind::Star),
    (Not, "The `!` operator.", Kind::Not),
    (OpenBrace, "An opening brace `{`.", Kind::Open(Delimiter::Brace)),
    (OpenBracket, "An open bracket `[`.", Kind::Open(Delimiter::Bracket)),
    (OpenParen, "An opening parenthesis `(`.", Kind::Open(Delimiter::Parenthesis)),
    (Or, "Or `||` operator.", Kind::PipePipe),
    (Pipe, "A pipe `|`.", Kind::Pipe),
    (Priv, "The `priv` keyword.", Kind::Priv),
    (Pub, "The `pub` keyword.", Kind::Pub),
    (Return, "The `return` keyword.", Kind::Return),
    (Rocket, "The rocket `=>`.", Kind::Rocket),
    (Scope, "A scope `::` declaration.", Kind::ColonColon),
    (Select, "The `select` keyword.", Kind::Select),
    (SelfType, "The `Self` type.", Kind::SelfType),
    (SelfValue, "The `self` keyword.", Kind::SelfValue),
    (SemiColon, "A semicolon `;`.", Kind::SemiColon),
    (Struct, "The `struct` keyword.", Kind::Struct),
    (Super, "The `super` keyword.", Kind::Super),
    (Try, "The `?` operator.", Kind::QuestionMark),
    (Underscore, "The underscore `_`.", Kind::Underscore),
    (Use, "The `use` keyword.", Kind::Use),
    (While, "The `while` keyword.", Kind::While),
    (Yield, "The `yield` keyword.", Kind::Yield),
}

#[cfg(test)]
mod tests {
    use crate::{ast, parse_all};

    #[test]
    fn test_expr() {
        parse_all::<ast::Expr>("foo[\"foo\"]").unwrap();
        parse_all::<ast::Expr>("foo.bar()").unwrap();
        parse_all::<ast::Expr>("var()").unwrap();
        parse_all::<ast::Expr>("var").unwrap();
        parse_all::<ast::Expr>("42").unwrap();
        parse_all::<ast::Expr>("1 + 2 / 3 - 4 * 1").unwrap();
        parse_all::<ast::Expr>("foo[\"bar\"]").unwrap();
        parse_all::<ast::Expr>("let var = 42").unwrap();
        parse_all::<ast::Expr>("let var = \"foo bar\"").unwrap();
        parse_all::<ast::Expr>("var[\"foo\"] = \"bar\"").unwrap();
        parse_all::<ast::Expr>("let var = objects[\"foo\"] + 1").unwrap();
        parse_all::<ast::Expr>("var = 42").unwrap();

        let expr = parse_all::<ast::Expr>(
            r#"
            if 1 { } else { if 2 { } else { } }
        "#,
        )
        .unwrap();

        if let ast::Expr::ExprIf(..) = expr {
        } else {
            panic!("not an if statement");
        }

        // Chained function calls.
        parse_all::<ast::Expr>("foo.bar.baz()").unwrap();
        parse_all::<ast::Expr>("foo[0][1][2]").unwrap();
        parse_all::<ast::Expr>("foo.bar()[0].baz()[1]").unwrap();

        parse_all::<ast::Expr>("42 is int::int").unwrap();
    }
}
