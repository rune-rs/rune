//! The core `std` module.

use crate::{ContextError, Module, Panic, Value, VmError};

/// Construct the `std` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.unit("unit")?;
    module.ty::<bool>()?;
    module.ty::<char>()?;
    module.ty::<u8>()?;

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
        Value::Object(object) => {
            object.take()?;
        }
        Value::UnitStruct(empty) => {
            empty.take()?;
        }
        Value::TupleStruct(tuple) => {
            tuple.take()?;
        }
        Value::Struct(object) => {
            object.take()?;
        }
        Value::UnitVariant(empty) => {
            empty.take()?;
        }
        Value::TupleVariant(tuple) => {
            tuple.take()?;
        }
        Value::StructVariant(object) => {
            object.take()?;
        }
        _ => (),
    }

    Ok::<(), VmError>(())
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
        Value::UnitVariant(empty) => empty.is_readable(),
        Value::TupleVariant(tuple) => tuple.is_readable(),
        Value::StructVariant(object) => object.is_readable(),
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
        Value::UnitVariant(empty) => empty.is_writable(),
        Value::TupleVariant(tuple) => tuple.is_writable(),
        Value::StructVariant(object) => object.is_writable(),
        _ => true,
    }
}
