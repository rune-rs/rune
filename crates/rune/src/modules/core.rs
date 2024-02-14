//! The core `std` module.

use crate as rune;
use crate::alloc::prelude::*;
use crate::compile;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Panic, Value, VmResult};
use crate::{ContextError, Module};

#[rune::module(::std)]
/// The Rune standard library.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?.with_unique("std");

    module.ty::<bool>()?.docs(["The primitive boolean type."])?;
    module
        .ty::<char>()?
        .docs(["The primitive character type."])?;
    module.ty::<u8>()?.docs(["The primitive byte type."])?;
    module.ty::<f64>()?.docs(["The primitive float type."])?;
    module.ty::<i64>()?.docs(["The primitive integer type."])?;

    module.function_meta(panic)?;
    module.function_meta(is_readable)?;
    module.function_meta(is_writable)?;

    module.macro_meta(stringify_macro)?;
    module.macro_meta(panic_macro)?;
    Ok(module)
}

/// Cause a vm panic with the given `message`.
///
/// A panic in Rune causes the current execution to unwind and terminate. The
/// panic will not be propagated into Rust, but will instead be signatted
/// through a `VmError`.
///
/// If you want to format a message, consider using the [panic!] macro.
#[rune::function]
fn panic(message: &str) -> VmResult<()> {
    VmResult::err(Panic::custom(vm_try!(message.try_to_owned())))
}

/// Test if the given `value` is readable.
///
/// A value is writable if can be acquired for shared access, such as producing
/// an immutable reference.
///
/// A value that is moved is no longer considered readable.
///
/// # Examples
///
/// ```rune
/// let value = Some(42);
/// assert!(is_readable(value));
/// let value2 = value.map(|v| v + 1);
/// assert!(!is_readable(value));
/// assert_eq!(value2, Some(43));
/// ```
#[rune::function]
fn is_readable(value: Value) -> bool {
    value.is_readable()
}

/// Test if the given `value` is writable.
///
/// A value is writable if can be acquired for exclusive access, such as
/// producing a mutable reference or taking ownership.
///
/// # Examples
///
/// ```rune
/// let value = Some(42);
/// assert!(is_writable(value));
/// let value2 = value.map(|v| v + 1);
/// assert!(!is_writable(value));
/// assert_eq!(value2, Some(43));
/// ```
#[rune::function]
fn is_writable(value: Value) -> bool {
    value.is_writable()
}

/// Stringify the given argument, causing it to expand to its underlying token
/// stream.
///
/// This can be used by macros to convert a stream of tokens into a readable
/// string.
#[rune::macro_(path = stringify)]
pub(crate) fn stringify_macro(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let lit = cx.stringify(stream)?.try_to_string()?;
    let lit = cx.lit(lit)?;
    Ok(quote!(#lit).into_token_stream(cx)?)
}

/// Cause a vm panic with a formatted message.
///
/// A panic in Rune causes the current execution to unwind and terminate. The
/// panic will not be propagated into Rust, but will instead be signatted
/// through a `VmError`.
#[rune::macro_(path = panic)]
pub(crate) fn panic_macro(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, cx.input_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(cx)?;
    Ok(quote!(::std::panic(#expanded)).into_token_stream(cx)?)
}
