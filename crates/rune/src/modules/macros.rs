//! `std::macros` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io

use crate as rune;
use crate::compile;
use crate::macros::{quote, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::{ContextError, Module};

/// Construct the `std::macros` module.
pub fn module() -> Result<Module, ContextError> {
    let mut builtins =
        Module::with_crate_item("std", ["macros", "builtin"])?.with_unique("std::macros::builtin");
    builtins.macro_meta(file)?;
    builtins.macro_meta(line)?;
    Ok(builtins)
}

/// Return the line in the current file.
///
/// # Examples
///
/// ```rune
/// println!("{}:{}: Something happened", file!(), line!());
/// ```
#[rune::macro_]
pub(crate) fn line(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut parser = Parser::from_token_stream(stream, cx.input_span());
    parser.eof()?;

    let stream = quote!(
        #[builtin]
        line!()
    );

    Ok(stream.into_token_stream(cx)?)
}

/// Return the name of the current file.
///
/// # Examples
///
/// ```rune
/// println!("{}:{}: Something happened", file!(), line!());
/// ```
#[rune::macro_]
pub(crate) fn file(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut parser = Parser::from_token_stream(stream, cx.input_span());
    parser.eof()?;

    let stream = quote!(
        #[builtin]
        file!()
    );

    Ok(stream.into_token_stream(cx)?)
}
