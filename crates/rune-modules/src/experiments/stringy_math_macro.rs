use rune::ast;
use rune::macros::resolve;
use rune::{quote, Parser, Spanned, TokenStream};

/// Implementation for the `stringy_math!` macro.
pub(crate) fn stringy_math(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match resolve(op)?.as_ref() {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => {
                return Err(From::from(runestick::SpannedError::msg(
                    op.span(),
                    "unsupported operation",
                )))
            }
        }
    }

    parser.eof()?;
    Ok(output)
}
