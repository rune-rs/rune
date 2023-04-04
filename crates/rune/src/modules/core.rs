//! The core `std` module.

use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Panic, Tuple, Value};
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

    module.function(["panic"], panic_impl)?;
    module.function(["is_readable"], is_readable)?;
    module.function(["is_writable"], is_writable)?;

    module.macro_(["stringify"], stringify_macro)?;
    module.macro_(["panic"], panic_macro)?;
    Ok(module)
}

fn panic_impl(m: &str) -> Result<(), Panic> {
    Err(Panic::custom(m.to_owned()))
}

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

/// Implementation for the `stringify!` macro.
pub(crate) fn stringify_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    use crate as rune;

    let lit = ctx.stringify(stream).to_string();
    let lit = ctx.lit(lit);
    Ok(quote!(#lit).into_token_stream(ctx))
}

pub(crate) fn panic_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    use crate as rune;

    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::panic(#expanded)).into_token_stream(ctx))
}
