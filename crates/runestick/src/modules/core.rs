//! The core `std` module.

use crate::{ContextError, Module, Panic, Stack, Value, VmError};
use std::io;
use std::io::Write as _;

/// Construct the `std` module.
pub fn module(io: bool) -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.unit(&["unit"])?;
    module.ty::<bool>()?;
    module.ty::<char>()?;
    module.ty::<u8>()?;

    if io {
        module.function(&["print"], print_impl)?;
        module.function(&["println"], println_impl)?;
        module.raw_fn(&["dbg"], dbg_impl)?;
    }

    module.function(&["panic"], panic_impl)?;
    module.function(&["drop"], drop_impl)?;
    module.function(&["is_readable"], is_readable)?;
    module.function(&["is_writable"], is_writable)?;
    Ok(module)
}

fn drop_impl(value: Value) -> Result<(), VmError> {
    match value {
        Value::Any(any) => {
            any.take()?;
        }
        Value::String(string) => {
            string.take()?;
        }
        Value::Bytes(bytes) => {
            bytes.take()?;
        }
        Value::Vec(vec) => {
            vec.take()?;
        }
        Value::Tuple(tuple) => {
            tuple.take()?;
        }
        Value::TypedTuple(tuple) => {
            tuple.take()?;
        }
        Value::TupleVariant(tuple) => {
            tuple.take()?;
        }
        Value::Object(object) => {
            object.take()?;
        }
        Value::TypedObject(object) => {
            object.take()?;
        }
        Value::VariantObject(object) => {
            object.take()?;
        }
        _ => (),
    }

    Ok::<(), VmError>(())
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for value in stack.drain_stack_top(args)? {
        writeln!(stdout, "{:?}", value).map_err(VmError::panic)?;
    }

    stack.push(Value::Unit);
    Ok(())
}

fn print_impl(m: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", m).map_err(Panic::custom)
}

fn println_impl(m: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{}", m).map_err(Panic::custom)
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
        Value::TypedTuple(tuple) => tuple.is_readable(),
        Value::TupleVariant(tuple) => tuple.is_readable(),
        Value::Object(object) => object.is_readable(),
        Value::TypedObject(object) => object.is_readable(),
        Value::VariantObject(object) => object.is_readable(),
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
        Value::TypedTuple(tuple) => tuple.is_writable(),
        Value::TupleVariant(tuple) => tuple.is_writable(),
        Value::Object(object) => object.is_writable(),
        Value::TypedObject(object) => object.is_writable(),
        Value::VariantObject(object) => object.is_writable(),
        _ => true,
    }
}
