//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::Value;

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty(&["unit"]).build::<()>()?;
    module.ty(&["bool"]).build::<bool>()?;
    module.ty(&["char"]).build::<char>()?;
    module.ty(&["byte"]).build::<u8>()?;

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

    Ok(module)
}
