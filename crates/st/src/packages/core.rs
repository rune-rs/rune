//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::ValuePtr;

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty(&["unit"]).build::<()>()?;
    module.ty(&["bool"]).build::<bool>()?;
    module.ty(&["char"]).build::<char>()?;

    module.raw_fn(&["dbg"], |vm, args| {
        for n in 0..args {
            match vm.pop().and_then(|v| vm.value_ref(v)) {
                Ok(value) => {
                    println!("{} = {:?}", n, value);
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        vm.push(ValuePtr::Unit);
        Ok(())
    })?;

    Ok(module)
}
