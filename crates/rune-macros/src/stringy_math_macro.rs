use rune::ast;
use rune::Resolve as _;
use rune::{quote, MacroContext, Parser, TokenStream};

/// Implementation for the `stringy_math!` macro.
pub(crate) fn stringy_math(
    ctx: &mut MacroContext,
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let mut output = quote!(ctx => 0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?.macro_resolve(ctx)?;

        match op.as_ref() {
            "add" => {
                let op = parser.parse::<ast::Expr>()?;
                output = quote!(ctx => (#output) + #op);
            }
            "sub" => {
                let op = parser.parse::<ast::Expr>()?;
                output = quote!(ctx => (#output) - #op);
            }
            "div" => {
                let op = parser.parse::<ast::Expr>()?;
                output = quote!(ctx => (#output) / #op);
            }
            "mul" => {
                let op = parser.parse::<ast::Expr>()?;
                output = quote!(ctx => (#output) * #op);
            }
            other => {
                return Err(runestick::Error::msg(format!(
                    "unsupported operation `{}`",
                    other
                )));
            }
        }
    }

    parser.parse_eof()?;
    Ok(output)
}
