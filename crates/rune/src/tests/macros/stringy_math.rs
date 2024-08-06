use crate as rune;
use crate::ast;
use crate::compile;
use crate::macros::{quote, MacroContext, TokenStream};
use crate::parse::Parser;

/// Implementation of the `stringy_math!` macro.
#[rune::macro_]
pub fn stringy_math(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, cx.input_span());

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match cx.resolve(op)? {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => return Err(compile::Error::msg(op, "unsupported operation")),
        }
    }

    parser.eof()?;
    Ok(output.into_token_stream(cx)?)
}
