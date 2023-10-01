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
    match value {
        Value::EmptyTuple => true,
        Value::Bool(_) => true,
        Value::Byte(_) => true,
        Value::Char(_) => true,
        Value::Integer(_) => true,
        Value::Float(_) => true,
        Value::Type(_) => true,
        Value::Ordering(_) => true,
        Value::String(value) => value.is_readable(),
        Value::Bytes(value) => value.is_readable(),
        Value::Vec(value) => value.is_readable(),
        Value::Tuple(value) => value.is_readable(),
        Value::Object(value) => value.is_readable(),
        Value::RangeFrom(value) => value.is_readable(),
        Value::RangeFull(value) => value.is_readable(),
        Value::RangeInclusive(value) => value.is_readable(),
        Value::RangeToInclusive(value) => value.is_readable(),
        Value::RangeTo(value) => value.is_readable(),
        Value::Range(value) => value.is_readable(),
        Value::ControlFlow(value) => value.is_readable(),
        Value::Future(value) => value.is_readable(),
        Value::Stream(value) => value.is_readable(),
        Value::Generator(value) => value.is_readable(),
        Value::GeneratorState(value) => value.is_readable(),
        Value::Option(value) => value.is_readable(),
        Value::Result(value) => value.is_readable(),
        Value::EmptyStruct(value) => value.is_readable(),
        Value::TupleStruct(value) => value.is_readable(),
        Value::Struct(value) => value.is_readable(),
        Value::Variant(value) => value.is_readable(),
        Value::Function(value) => value.is_readable(),
        Value::Format(_) => true,
        Value::Iterator(value) => value.is_readable(),
        Value::Any(value) => value.is_readable(),
    }
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
    match value {
        Value::EmptyTuple => true,
        Value::Bool(_) => true,
        Value::Byte(_) => true,
        Value::Char(_) => true,
        Value::Integer(_) => true,
        Value::Float(_) => true,
        Value::Type(_) => true,
        Value::Ordering(_) => true,
        Value::String(value) => value.is_writable(),
        Value::Bytes(value) => value.is_writable(),
        Value::Vec(value) => value.is_writable(),
        Value::Tuple(value) => value.is_writable(),
        Value::Object(value) => value.is_writable(),
        Value::RangeFrom(value) => value.is_writable(),
        Value::RangeFull(value) => value.is_writable(),
        Value::RangeInclusive(value) => value.is_writable(),
        Value::RangeToInclusive(value) => value.is_writable(),
        Value::RangeTo(value) => value.is_writable(),
        Value::Range(value) => value.is_writable(),
        Value::ControlFlow(value) => value.is_writable(),
        Value::Future(value) => value.is_writable(),
        Value::Stream(value) => value.is_writable(),
        Value::Generator(value) => value.is_writable(),
        Value::GeneratorState(value) => value.is_writable(),
        Value::Option(value) => value.is_writable(),
        Value::Result(value) => value.is_writable(),
        Value::EmptyStruct(value) => value.is_writable(),
        Value::TupleStruct(value) => value.is_writable(),
        Value::Struct(value) => value.is_writable(),
        Value::Variant(value) => value.is_writable(),
        Value::Function(value) => value.is_writable(),
        Value::Format(_) => false,
        Value::Iterator(value) => value.is_writable(),
        Value::Any(value) => value.is_writable(),
    }
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
