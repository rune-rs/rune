//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::{Array, Object, Value, ValuePtr};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty::<()>("unit")?;
    module.ty::<bool>("bool")?;
    module.ty::<char>("char")?;
    module.ty::<Array<Value>>("Array")?;
    module.ty::<Object<Value>>("Object")?;

    module.raw_free_fn(&["dbg"], |vm, args| {
        for n in 0..args {
            match vm.managed_pop() {
                Ok(value) => {
                    println!("{} = {:?}", n, vm.value_ref(value));
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        vm.managed_push(ValuePtr::Unit)?;
        Ok(())
    })?;

    Ok(module)
}
