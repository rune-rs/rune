//! Built-in macros.

use crate as rune;
use crate::compile::{self, ErrorKind};
use crate::macros::{quote, MacroContext, TokenStream};
use crate::{ContextError, Module};

/// Built-in macros.
#[rune::module(::std::macros::builtin)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?.with_unique("std::macros::builtin");
    m.macro_meta(file)?;
    m.macro_meta(line)?;
    Ok(m)
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

    expect_no_input(stream)?;

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

    expect_no_input(stream)?;

    let stream = quote!(
        #[builtin]
        file!()
    );

    Ok(stream.into_token_stream(cx)?)
}

/// Refuse the input of a macro which takes none.
///
/// The tokens are looked at rather than parsed, since a macro which has nothing
/// to parse has no business building a syntax tree to find that out.
fn expect_no_input(stream: &TokenStream) -> compile::Result<()> {
    let Some(token) = stream.into_iter().next() else {
        return Ok(());
    };

    Err(compile::Error::new(
        token.span,
        ErrorKind::ExpectedEof { actual: token.kind },
    ))
}
