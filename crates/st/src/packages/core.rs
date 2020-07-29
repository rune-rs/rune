//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::functions::{Functions, RegisterError};
use crate::value::ValuePtr;

/// Install the core package into the given functions namespace.
pub fn install(functions: &mut Functions) -> Result<(), RegisterError> {
    let module = functions.global_module_mut();

    module.raw_fn("dbg", |vm, args| {
        for n in 0..args {
            match vm.managed_pop() {
                Ok(value) => {
                    println!("{} = {:?}", n, vm.to_value_ref(value));
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        vm.managed_push(ValuePtr::Unit)?;
        Ok(())
    })?;

    Ok(())
}
