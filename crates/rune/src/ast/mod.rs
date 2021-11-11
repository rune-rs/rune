//! AST for the Rune language.

use crate::{Parse, ParseError, Parser, Peek};
use runestick::Span;

#[macro_use]
/// Generated modules.
pub mod generated;

macro_rules! expr_parse {
    ($ty:ident, $local:ty, $expected:literal) => {
        impl crate::Parse for $local {
            fn parse(p: &mut crate::Parser<'_>) -> Result<Self, crate::ParseError> {
                let t = p.tok_at(0)?;

                match crate::ast::Expr::parse(p)? {
                    crate::ast::Expr::$ty(expr) => Ok(*expr),
                    _ => Err(crate::ParseError::expected(&t, $expected)),
                }
            }
        }
    };
}

macro_rules! item_parse {
    ($ty:ident, $local:ty, $expected:literal) => {
        impl crate::Parse for $local {
            fn parse(p: &mut crate::Parser<'_>) -> Result<Self, crate::ParseError> {
                let t = p.tok_at(0)?;

                match crate::ast::Item::parse(p)? {
                    crate::ast::Item::$ty(item) => Ok(*item),
                    _ => Err(crate::ParseError::expected(&t, $expected)),
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
mod force_semi;
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
mod stmt;
mod token;
pub(super) mod utils;
mod vis;

pub use self::attribute::Attribute;
pub use self::block::Block;
pub use self::condition::Condition;
pub use self::expr::{Expr, ExprWithoutBinary};
pub use self::expr_assign::ExprAssign;
pub use self::expr_await::ExprAwait;
pub use self::expr_binary::{BinOp, ExprBinary};
pub use self::expr_block::ExprBlock;
pub use self::expr_break::{ExprBreak, ExprBreakValue};
pub use self::expr_call::ExprCall;
pub use self::expr_closure::ExprClosure;
pub use self::expr_continue::ExprContinue;
pub use self::expr_field_access::{ExprField, ExprFieldAccess};
pub use self::expr_for::ExprFor;
pub use self::expr_group::ExprGroup;
pub use self::expr_if::{ExprElse, ExprElseIf, ExprIf};
pub use self::expr_index::ExprIndex;
pub use self::expr_let::ExprLet;
pub use self::expr_lit::ExprLit;
pub use self::expr_loop::ExprLoop;
pub use self::expr_match::{ExprMatch, ExprMatchBranch};
pub use self::expr_object::{AnonExprObject, ExprObject, FieldAssign, ObjectIdent, ObjectKey};
pub use self::expr_range::{ExprRange, ExprRangeLimits};
pub use self::expr_return::ExprReturn;
pub use self::expr_select::{ExprSelect, ExprSelectBranch};
pub use self::expr_try::ExprTry;
pub use self::expr_tuple::ExprTuple;
pub use self::expr_unary::{ExprUnary, UnOp};
pub use self::expr_vec::ExprVec;
pub use self::expr_while::ExprWhile;
pub use self::expr_yield::ExprYield;
pub use self::file::File;
pub use self::fn_arg::FnArg;
pub use self::force_semi::ForceSemi;
pub use self::generated::Kind;
pub use self::grouped::{AngleBracketed, Braced, Bracketed, Parenthesized};
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
pub use self::lit_str::LitStr;
pub use self::local::Local;
pub use self::macro_call::MacroCall;
pub use self::pat::{Pat, PatBinding, PatLit, PatObject, PatPath, PatTuple, PatVec};
pub use self::path::{Path, PathKind, PathSegment};
pub use self::stmt::{ItemOrExpr, Stmt, StmtSortKey};
pub use self::token::{
    BuiltIn, CopySource, Delimiter, Number, NumberBase, NumberSource, NumberText, StrSource,
    StrText, StringSource, Token,
};
pub use self::vis::Visibility;

macro_rules! decl_tokens {
    ($(($parser:ident, $name:literal, $doc:expr, $($kind:tt)*),)*) => {
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
                    let token = parser.next()?;

                    match token.kind {
                        $($kind)* => Ok(Self {
                            token,
                        }),
                        _ => Err(ParseError::expected(&token, $name)),
                    }
                }
            }

            impl Peek for $parser {
                fn peek(p: &mut $crate::parsing::Peeker<'_>) -> bool {
                    matches!(p.nth(0), $($kind)*)
                }
            }

            impl crate::ToTokens for $parser {
                fn to_tokens(&self, _: &crate::MacroContext, stream: &mut crate::TokenStream) {
                    stream.push(self.token);
                }
            }
        )*
    }
}

decl_tokens! {
    (CloseBrace, "An closing brace `}`.", "closing brace", Kind::Close(Delimiter::Brace)),
    (CloseBracket, "An open bracket `]`.", "closing bracket", Kind::Close(Delimiter::Bracket)),
    (CloseParen, "An closing parenthesis `)`.", "closing parenthesis", Kind::Close(Delimiter::Parenthesis)),
    (OpenBrace, "An opening brace `{`.", "opening brace", Kind::Open(Delimiter::Brace)),
    (OpenBracket, "An open bracket `[`.", "opening bracket", Kind::Open(Delimiter::Bracket)),
    (OpenParen, "An opening parenthesis `(`.", "opening parenthesis", Kind::Open(Delimiter::Parenthesis)),
}

#[cfg(test)]
mod tests {
    use crate::{ast, parse_all_without_source};

    #[test]
    fn test_expr() {
        parse_all_without_source::<ast::Expr>("foo[\"foo\"]").unwrap();
        parse_all_without_source::<ast::Expr>("foo.bar()").unwrap();
        parse_all_without_source::<ast::Expr>("var()").unwrap();
        parse_all_without_source::<ast::Expr>("var").unwrap();
        parse_all_without_source::<ast::Expr>("42").unwrap();
        parse_all_without_source::<ast::Expr>("1 + 2 / 3 - 4 * 1").unwrap();
        parse_all_without_source::<ast::Expr>("foo[\"bar\"]").unwrap();
        parse_all_without_source::<ast::Expr>("let var = 42").unwrap();
        parse_all_without_source::<ast::Expr>("let var = \"foo bar\"").unwrap();
        parse_all_without_source::<ast::Expr>("var[\"foo\"] = \"bar\"").unwrap();
        parse_all_without_source::<ast::Expr>("let var = objects[\"foo\"] + 1").unwrap();
        parse_all_without_source::<ast::Expr>("var = 42").unwrap();

        let expr = parse_all_without_source::<ast::Expr>(
            r#"
            if 1 { } else { if 2 { } else { } }
        "#,
        )
        .unwrap();

        if let ast::Expr::If(..) = expr {
        } else {
            panic!("not an if statement");
        }

        // Chained function calls.
        parse_all_without_source::<ast::Expr>("foo.bar.baz()").unwrap();
        parse_all_without_source::<ast::Expr>("foo[0][1][2]").unwrap();
        parse_all_without_source::<ast::Expr>("foo.bar()[0].baz()[1]").unwrap();

        parse_all_without_source::<ast::Expr>("42 is int::int").unwrap();
    }
}
