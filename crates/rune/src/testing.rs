//! Internal testing module.

use std::fmt;

use crate::macros::{MacroContext, ToTokens, TokenStream};
use crate::parse::{Parse, Parser};
use crate::SourceId;

/// Function used during parse testing to take the source, parse it as the given
/// type, tokenize it using [ToTokens], and parse the token stream.
///
/// The results should be identical.
pub fn roundtrip<T>(source: &str) -> T
where
    T: Parse + ToTokens + PartialEq + Eq + fmt::Debug,
{
    let source_id = SourceId::empty();

    let mut parser = Parser::new(source, source_id);
    let ast = parser.parse::<T>().expect("first parse");
    parser.eof().expect("first parse eof");

    let ast2 = MacroContext::test(|ctx| {
        let mut stream = TokenStream::new();
        ast.to_tokens(ctx, &mut stream);
        let mut parser = Parser::from_token_stream(&stream, ctx.stream_span());
        let ast2 = parser.parse::<T>().expect("second parse");
        parser.eof().expect("second parse eof");
        ast2
    });

    assert_eq!(ast, ast2);
    ast
}
