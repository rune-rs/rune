//! AST for the Rune language.

use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{Delimiter, Kind, Token};
use crate::traits::{Parse, Peek, Resolve};
use runestick::unit::Span;

mod call_fn;
mod call_instance_fn;
mod condition;
mod decl_enum;
mod decl_file;
mod decl_fn;
mod decl_struct;
mod decl_use;
mod expr;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_else;
mod expr_else_if;
mod expr_for;
mod expr_group;
mod expr_if;
mod expr_index_get;
mod expr_index_set;
mod expr_let;
mod expr_loop;
mod expr_match;
mod expr_return;
mod expr_select;
mod expr_try;
mod expr_unary;
mod expr_while;
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
mod parenthesized;
mod pat;
mod pat_object;
mod pat_tuple;
mod pat_vec;
mod path;
pub(super) mod utils;

pub use self::call_fn::CallFn;
pub use self::call_instance_fn::CallInstanceFn;
pub use self::condition::Condition;
pub use self::decl_enum::DeclEnum;
pub use self::decl_file::DeclFile;
pub use self::decl_fn::DeclFn;
pub use self::decl_struct::{DeclStruct, DeclStructBody, EmptyBody, StructBody, TupleBody};
pub use self::decl_use::DeclUse;
pub use self::expr::Expr;
pub use self::expr_await::ExprAwait;
pub use self::expr_binary::{BinOp, ExprBinary};
pub use self::expr_block::ExprBlock;
pub use self::expr_break::{ExprBreak, ExprBreakValue};
pub use self::expr_else::ExprElse;
pub use self::expr_else_if::ExprElseIf;
pub use self::expr_for::ExprFor;
pub use self::expr_group::ExprGroup;
pub use self::expr_if::ExprIf;
pub use self::expr_index_get::ExprIndexGet;
pub use self::expr_index_set::ExprIndexSet;
pub use self::expr_let::ExprLet;
pub use self::expr_loop::ExprLoop;
pub use self::expr_match::{ExprMatch, ExprMatchBranch};
pub use self::expr_return::ExprReturn;
pub use self::expr_select::ExprSelect;
pub use self::expr_try::ExprTry;
pub use self::expr_unary::{ExprUnary, UnaryOp};
pub use self::expr_while::ExprWhile;
pub use self::lit_bool::LitBool;
pub use self::lit_byte::LitByte;
pub use self::lit_byte_str::LitByteStr;
pub use self::lit_char::LitChar;
pub use self::lit_number::{LitNumber, Number};
pub use self::lit_object::{LitObject, LitObjectFieldAssign, LitObjectIdent, LitObjectKey};
pub use self::lit_str::LitStr;
pub use self::lit_template::{LitTemplate, Template, TemplateComponent};
pub use self::lit_tuple::LitTuple;
pub use self::lit_unit::LitUnit;
pub use self::lit_vec::LitVec;
pub use self::parenthesized::Parenthesized;
pub use self::pat::Pat;
pub use self::pat_object::{PatObject, PatObjectItem};
pub use self::pat_tuple::PatTuple;
pub use self::pat_vec::PatVec;
pub use self::path::Path;

macro_rules! decl_tokens {
    ($(($parser:ident, $($kind:tt)*),)*) => {
        $(
            /// Helper parser for a specifik token kind
            #[derive(Debug, Clone, Copy)]
            pub struct $parser {
                /// Associated token.
                pub token: Token,
            }

            impl $parser {
                /// Access the span of the token.
                pub fn span(&self) -> Span {
                    self.token.span
                }
            }

            impl Parse for $parser {
                fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
                    let token = parser.token_next()?;

                    match token.kind {
                        $($kind)* => Ok(Self {
                            token,
                        }),
                        _ => Err(ParseError::TokenMismatch {
                            expected: $($kind)*,
                            actual: token.kind,
                            span: token.span,
                        }),
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
        )*
    }
}

decl_tokens! {
    (Fn, Kind::Fn),
    (Enum, Kind::Enum),
    (Struct, Kind::Struct),
    (If, Kind::If),
    (Match, Kind::Match),
    (Else, Kind::Else),
    (Let, Kind::Let),
    (Ident, Kind::Ident),
    (Label, Kind::Label),
    (OpenParen, Kind::Open(Delimiter::Parenthesis)),
    (CloseParen, Kind::Close(Delimiter::Parenthesis)),
    (OpenBrace, Kind::Open(Delimiter::Brace)),
    (CloseBrace, Kind::Close(Delimiter::Brace)),
    (OpenBracket, Kind::Open(Delimiter::Bracket)),
    (CloseBracket, Kind::Close(Delimiter::Bracket)),
    (Underscore, Kind::Underscore),
    (Comma, Kind::Comma),
    (Colon, Kind::Colon),
    (Dot, Kind::Dot),
    (SemiColon, Kind::SemiColon),
    (Eq, Kind::Eq),
    (Use, Kind::Use),
    (Scope, Kind::Scope),
    (While, Kind::While),
    (Loop, Kind::Loop),
    (For, Kind::For),
    (In, Kind::In),
    (Break, Kind::Break),
    (Return, Kind::Return),
    (Star, Kind::Mul),
    (Rocket, Kind::Rocket),
    (Hash, Kind::Hash),
    (DotDot, Kind::DotDot),
    (Await, Kind::Await),
    (Select, Kind::Select),
    (Try, Kind::Try),
}

impl<'a> Resolve<'a> for Ident {
    type Output = &'a str;

    fn resolve(&self, source: Source<'a>) -> Result<&'a str, ParseError> {
        source.source(self.token.span)
    }
}

impl<'a> Resolve<'a> for Label {
    type Output = &'a str;

    fn resolve(&self, source: Source<'a>) -> Result<&'a str, ParseError> {
        source.source(self.token.span.trim_start(1))
    }
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

        if let ast::Expr::ExprIf(..) = expr.item {
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
