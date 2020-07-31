//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::functions::{Module, RegisterError};
use crate::value::{Array, Object, Value, ValuePtr};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["core"]);

    module.register_type::<()>("unit")?;
    module.register_type::<bool>("bool")?;
    module.register_type::<char>("char")?;
    module.register_type::<i64>("int")?;
    module.register_type::<f64>("float")?;
    module.register_type::<Array<Value>>("Array")?;
    module.register_type::<Object<Value>>("Object")?;

    module.raw_fn("dbg", |vm, args| {
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
