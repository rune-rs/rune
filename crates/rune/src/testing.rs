use crate::SourceId;

/// Function used during parse testing to take the source, parse it as the given
/// type, tokenize it using [ToTokens][crate::macros::ToTokens], and parse the
/// token stream.
///
/// The results should be identical.
pub fn roundtrip<T>(source: &str) -> T
where
    T: crate::parsing::Parse + crate::macros::ToTokens + PartialEq + Eq + std::fmt::Debug,
{
    let source_id = SourceId::empty();

    let mut parser = crate::parsing::Parser::new(source, source_id);
    let ast = parser.parse::<T>().expect("first parse");
    parser.eof().expect("first parse eof");

    let ast2 = crate::macros::MacroContext::test(|ctx| {
        let mut token_stream = crate::macros::TokenStream::new();

        ast.to_tokens(ctx, &mut token_stream);
        let mut parser =
            crate::parsing::Parser::from_token_stream(&token_stream, ctx.stream_span());
        let ast2 = parser.parse::<T>().expect("second parse");
        parser.eof().expect("second parse eof");
        ast2
    });

    assert_eq!(ast, ast2);
    ast
}
