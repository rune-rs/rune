//! AST for the Rune language.

use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek};
use runestick::Span;

mod block;
mod condition;
mod expr;
mod expr_async;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_else;
mod expr_else_if;
mod expr_field_access;
mod expr_for;
mod expr_group;
mod expr_if;
mod expr_index_get;
mod expr_index_set;
mod expr_is;
mod expr_is_not;
mod expr_let;
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
mod ident;
mod item;
mod item_enum;
mod item_fn;
mod item_impl;
mod item_mod;
mod item_struct;
mod item_use;
mod label;
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
mod macro_call;
mod parenthesized;
mod pat;
mod pat_object;
mod pat_path;
mod pat_tuple;
mod pat_vec;
mod path;
mod stmt;
mod token;
pub(super) mod utils;

pub use self::block::Block;
pub use self::condition::Condition;
pub use self::expr::Expr;
pub use self::expr_async::ExprAsync;
pub use self::expr_await::ExprAwait;
pub use self::expr_binary::{BinOp, ExprBinary};
pub use self::expr_block::ExprBlock;
pub use self::expr_break::{ExprBreak, ExprBreakValue};
pub use self::expr_call::ExprCall;
pub use self::expr_closure::ExprClosure;
pub use self::expr_else::ExprElse;
pub use self::expr_else_if::ExprElseIf;
pub use self::expr_field_access::{ExprField, ExprFieldAccess};
pub use self::expr_for::ExprFor;
pub use self::expr_group::ExprGroup;
pub use self::expr_if::ExprIf;
pub use self::expr_index_get::ExprIndexGet;
pub use self::expr_index_set::ExprIndexSet;
pub use self::expr_is::ExprIs;
pub use self::expr_is_not::ExprIsNot;
pub use self::expr_let::ExprLet;
pub use self::expr_loop::ExprLoop;
pub use self::expr_match::{ExprMatch, ExprMatchBranch};
pub use self::expr_return::ExprReturn;
pub use self::expr_select::ExprSelect;
pub use self::expr_try::ExprTry;
pub use self::expr_unary::{ExprUnary, UnaryOp};
pub use self::expr_while::ExprWhile;
pub use self::expr_yield::ExprYield;
pub use self::file::File;
pub use self::fn_arg::FnArg;
pub use self::ident::Ident;
pub use self::item::Item;
pub use self::item_enum::{ItemEnum, ItemEnumVariant};
pub use self::item_fn::ItemFn;
pub use self::item_impl::ItemImpl;
pub use self::item_mod::{ItemMod, ItemModBody};
pub use self::item_struct::{ItemStruct, ItemStructBody, StructBody, TupleBody};
pub use self::item_use::{ItemUse, ItemUseComponent};
pub use self::label::Label;
pub use self::lit_bool::LitBool;
pub use self::lit_byte::LitByte;
pub use self::lit_byte_str::LitByteStr;
pub use self::lit_char::LitChar;
pub use self::lit_number::LitNumber;
pub use self::lit_object::{LitObject, LitObjectFieldAssign, LitObjectIdent, LitObjectKey};
pub use self::lit_str::LitStr;
pub use self::lit_template::{LitTemplate, Template, TemplateComponent};
pub use self::lit_tuple::LitTuple;
pub use self::lit_unit::LitUnit;
pub use self::lit_vec::LitVec;
pub use self::macro_call::MacroCall;
pub use self::parenthesized::Parenthesized;
pub use self::pat::Pat;
pub use self::pat_object::{PatObject, PatObjectItem};
pub use self::pat_path::PatPath;
pub use self::pat_tuple::PatTuple;
pub use self::pat_vec::PatVec;
pub use self::path::Path;
pub use self::stmt::Stmt;
pub use self::token::{
    CopySource, Delimiter, Kind, LitByteStrSource, LitByteStrSourceText, LitStrSource,
    LitStrSourceText, Number, NumberBase, NumberSource, NumberSourceText, StringSource, Token,
};

macro_rules! decl_tokens {
    ($(($parser:ident, $doc:expr, $($kind:tt)*),)*) => {
        $(
            #[doc = $doc]
            #[derive(Debug, Clone, Copy)]
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

            impl crate::IntoTokens for $parser {
                fn into_tokens(&self, _: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
                    stream.push(self.token);
                }
            }
        )*
    }
}

decl_tokens! {
    (OpenParen, "An opening parenthesis `(`.", Kind::Open(Delimiter::Parenthesis)),
    (CloseParen, "An closing parenthesis `)`.", Kind::Close(Delimiter::Parenthesis)),
    (OpenBrace, "An opening brace `{`.", Kind::Open(Delimiter::Brace)),
    (CloseBrace, "An closing brace `}`.", Kind::Close(Delimiter::Brace)),
    (OpenBracket, "An open bracket `[`.", Kind::Open(Delimiter::Bracket)),
    (CloseBracket, "An open bracket `]`.", Kind::Close(Delimiter::Bracket)),
    (Self_, "The `self` keyword.", Kind::Self_),
    (Fn, "The `fn` keyword.", Kind::Fn),
    (Is, "The `is` keyword.", Kind::Is),
    (Not, "The `!` operator.", Kind::Not),
    (Enum, "The `enum` keyword.", Kind::Enum),
    (Struct, "The `struct` keyword.", Kind::Struct),
    (If, "The `if` keyword.", Kind::If),
    (Match, "The `match` keyword.", Kind::Match),
    (Else, "The `else` keyword.", Kind::Else),
    (Let, "The `let` keyword.", Kind::Let),
    (Underscore, "The underscore `_`.", Kind::Underscore),
    (Comma, "A comma `,`.", Kind::Comma),
    (Colon, "A colon `:`.", Kind::Colon),
    (Dot, "A dot `.`.", Kind::Dot),
    (SemiColon, "A semicolon `;`.", Kind::SemiColon),
    (Eq, "An equals sign `=`.", Kind::Eq),
    (Use, "The `use` keyword.", Kind::Use),
    (Scope, "A scope `::` declaration.", Kind::ColonColon),
    (While, "The `while` keyword.", Kind::While),
    (Loop, "The `loop` keyword.", Kind::Loop),
    (For, "The `for` keyword.", Kind::For),
    (In, "The `in` keyword.", Kind::In),
    (Break, "The `break` keyword.", Kind::Break),
    (Yield, "The `yield` keyword.", Kind::Yield),
    (Return, "The `return` keyword.", Kind::Return),
    (Rocket, "The rocket `=>`.", Kind::Rocket),
    (Hash, "The hash `#`.", Kind::Pound),
    (DotDot, "Two dots `..`.", Kind::DotDot),
    (Await, "The `await` keyword.", Kind::Await),
    (Async, "The `async` keyword.", Kind::Async),
    (Select, "The `select` keyword.", Kind::Select),
    (Default, "The `default` keyword.", Kind::Default),
    (Try, "The `?` operator.", Kind::QuestionMark),
    (Pipe, "A pipe `|`.", Kind::Pipe),
    (And, "And `&&` operator.", Kind::AmpAmp),
    (Or, "Or `||` operator.", Kind::PipePipe),
    (Impl, "The `impl` keyword", Kind::Impl),
    (Mul, "Multiply `*` operator.", Kind::Star),
    (Mod, "The `mod` keyword.", Kind::Mod),
    (Bang, "The `!` operator.", Kind::Bang),
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
