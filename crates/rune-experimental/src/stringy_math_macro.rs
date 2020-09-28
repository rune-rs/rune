use rune::ast;
use rune::Resolve as _;
use rune::{quote, MacroContext, Parser, Spanned, TokenStream};

/// Implementation for the `stringy_math!` macro.
pub(crate) fn stringy_math(
    ctx: &mut MacroContext,
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let mut output = quote!(ctx => 0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match op.macro_resolve(ctx)?.as_ref() {
            "add" => quote!(ctx => (#output) + #arg),
            "sub" => quote!(ctx => (#output) - #arg),
            "div" => quote!(ctx => (#output) / #arg),
            "mul" => quote!(ctx => (#output) * #arg),
            _ => {
                return Err(From::from(runestick::SpannedError::msg(
                    op.span(),
                    "unsupported operation",
                )));
            }
        }
    }

    parser.parse_eof()?;
    Ok(output)
}
