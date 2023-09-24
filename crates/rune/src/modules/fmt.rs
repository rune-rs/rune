//! The `std::fmt` module.

use core::fmt;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::compile;
use crate::macros::{FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Format, Formatter, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::fmt` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["fmt"])?.with_unique("std::fmt");
    module.ty::<Format>()?;
    module.ty::<Formatter>()?;
    module.ty::<fmt::Error>()?;
    module.function_meta(fmt_error_string_display)?;
    module.macro_meta(format)?;
    Ok(module)
}

#[rune::function(instance, protocol = STRING_DISPLAY)]
fn fmt_error_string_display(error: &fmt::Error, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{}", error);
    VmResult::Ok(())
}

/// Format a string using a format specifier.
///
/// # Examples
///
/// ```rune
/// let who = "World";
/// let string = format!("Hello {}", who);
/// assert_eq!(string, "Hello World");
/// ```
#[rune::macro_(path = format)]
pub(crate) fn format(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, cx.input_span());
    let args = p.parse::<FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(cx)?;
    Ok(expanded.into_token_stream(cx)?)
}
