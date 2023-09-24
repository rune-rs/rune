//! Internal testing module.

use core::fmt;

use crate::macros::{self, ToTokens, TokenStream};
use crate::parse::{Parse, Parser};
use crate::SourceId;

/// Function used during parse testing to take the source, parse it as the given
/// type, tokenize it using [ToTokens], and parse the token stream.
///
/// The results should be identical.
pub(crate) fn rt<T>(source: &str) -> T
where
    T: Parse + ToTokens + PartialEq + Eq + fmt::Debug,
{
    rt_with(source, false)
}

pub(crate) fn rt_with<T>(source: &str, shebang: bool) -> T
where
    T: Parse + ToTokens + PartialEq + Eq + fmt::Debug,
{
    macro_rules! expect {
        ($expr:expr, $what:expr) => {
            match $expr {
                Ok(ast) => ast,
                Err(error) => {
                    panic!("{}: {error}:\n{source}", $what);
                }
            }
        };
    }

    let source_id = SourceId::empty();

    let mut parser = Parser::new(source, source_id, shebang);

    let ast = expect!(parser.parse::<T>(), "first parse");
    expect!(parser.eof(), "First parse EOF");

    let ast2 = macros::test(|cx| {
        let mut stream = TokenStream::new();
        ast.to_tokens(cx, &mut stream)?;
        let mut parser = Parser::from_token_stream(&stream, cx.input_span());
        let ast2 = expect!(parser.parse::<T>(), "Second parse");
        expect!(parser.eof(), "Second parse EOF");
        Ok(ast2)
    })
    .unwrap();

    assert_eq!(ast, ast2);
    ast
}
