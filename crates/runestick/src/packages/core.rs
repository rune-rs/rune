//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::{ContextError, Module, Panic, Value, ValueError};
use std::io;
use std::io::Write as _;

fn drop_impl(value: Value) -> Result<(), ValueError> {
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
        Value::VariantTuple(tuple) => {
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

    Ok::<(), ValueError>(())
}

fn is_readable(value: Value) -> bool {
    match value {
        Value::Any(any) => any.is_readable(),
        Value::String(string) => string.is_readable(),
        Value::Bytes(bytes) => bytes.is_readable(),
        Value::Vec(vec) => vec.is_readable(),
        Value::Tuple(tuple) => tuple.is_readable(),
        Value::TypedTuple(tuple) => tuple.is_readable(),
        Value::VariantTuple(tuple) => tuple.is_readable(),
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
        Value::VariantTuple(tuple) => tuple.is_writable(),
        Value::Object(object) => object.is_writable(),
        Value::TypedObject(object) => object.is_writable(),
        Value::VariantObject(object) => object.is_writable(),
        _ => true,
    }
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.unit(&["unit"])?;
    module.ty(&["bool"]).build::<bool>()?;
    module.ty(&["char"]).build::<char>()?;
    module.ty(&["byte"]).build::<u8>()?;

    module.function(&["print"], |message: &str| {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        write!(stdout, "{}", message)
    })?;

    module.function(&["println"], |message: &str| {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(stdout, "{}", message)
    })?;

    module.function(&["panic"], |message: &str| {
        Err::<(), _>(Panic::custom(message.to_owned()))
    })?;

    module.raw_fn(&["dbg"], |stack, args| {
        for n in 0..args {
            match stack.pop() {
                Ok(value) => {
                    println!("{} = {:?}", n, value);
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        stack.push(Value::Unit);
        Ok(())
    })?;

    module.function(&["drop"], drop_impl)?;
    module.function(&["is_readable"], is_readable)?;
    module.function(&["is_writable"], is_writable)?;
    Ok(module)
}
