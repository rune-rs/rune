//! The core `std` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::compile;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Panic, Tuple, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("std").with_unique("std");

    module.unit("unit")?;
    module.ty::<bool>()?;
    module.ty::<char>()?;
    module.ty::<u8>()?;
    module.ty::<f64>()?;
    module.ty::<i64>()?;
    module.ty::<Tuple>()?;

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
    VmResult::err(Panic::custom(message.to_owned()))
}

/// Test if the given `value` is readable.
#[rune::function]
fn is_readable(value: Value) -> bool {
    match value {
        Value::Any(any) => any.is_readable(),
        Value::String(string) => string.is_readable(),
        Value::Bytes(bytes) => bytes.is_readable(),
        Value::Vec(vec) => vec.is_readable(),
        Value::Tuple(tuple) => tuple.is_readable(),
        Value::Object(object) => object.is_readable(),
        Value::UnitStruct(empty) => empty.is_readable(),
        Value::TupleStruct(tuple) => tuple.is_readable(),
        Value::Struct(object) => object.is_readable(),
        Value::Variant(variant) => variant.is_readable(),
        _ => true,
    }
}

/// Test if the given `value` is writable.
#[rune::function]
fn is_writable(value: Value) -> bool {
    match value {
        Value::Any(any) => any.is_writable(),
        Value::String(string) => string.is_writable(),
        Value::Bytes(bytes) => bytes.is_writable(),
        Value::Vec(vec) => vec.is_writable(),
        Value::Tuple(tuple) => tuple.is_writable(),
        Value::Object(object) => object.is_writable(),
        Value::UnitStruct(empty) => empty.is_writable(),
        Value::TupleStruct(tuple) => tuple.is_writable(),
        Value::Struct(object) => object.is_writable(),
        Value::Variant(variant) => variant.is_writable(),
        _ => true,
    }
}

/// Stringify the given argument, causing it to expand to its underlying token
/// stream.
///
/// This can be used by macros to convert a stream of tokens into a readable
/// string.
#[rune::macro_(path = stringify)]
pub(crate) fn stringify_macro(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let lit = ctx.stringify(stream).to_string();
    let lit = ctx.lit(lit);
    Ok(quote!(#lit).into_token_stream(ctx))
}

/// Cause a vm panic with a formatted message.
///
/// A panic in Rune causes the current execution to unwind and terminate. The
/// panic will not be propagated into Rust, but will instead be signatted
/// through a `VmError`.
#[rune::macro_(path = panic)]
pub(crate) fn panic_macro(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.input_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::panic(#expanded)).into_token_stream(ctx))
}
