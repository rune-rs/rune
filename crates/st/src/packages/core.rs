//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::functions::{Error, Functions};
use crate::value::ValueRef;

/// Install the core package into the given functions namespace.
pub fn install(functions: &mut Functions) -> Result<(), Error> {
    functions.register_raw("dbg", |vm, args| {
        for n in 0..args {
            match vm.managed_pop() {
                Ok(value) => {
                    println!("{} = {:?}", n, vm.to_owned_value(value));
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        vm.managed_push(ValueRef::Unit)?;
        Ok(())
    })?;

    Ok(())
}
