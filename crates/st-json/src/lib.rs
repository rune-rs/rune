//! The json package, providing access to functions to serialize and deserialize
//! json.

use st::packages::bytes::Bytes;
use st::{Module, RegisterError, ValuePtr, Vm, VmError};

fn from_bytes(vm: &mut Vm, args: usize) -> Result<(), VmError> {
    if args != 1 {
        return Err(VmError::ArgumentCountMismatch {
            actual: args,
            expected: 1,
        });
    }

    let bytes = vm.pop_decode::<Bytes>()?;

    let value_ptr: ValuePtr = st::tls::inject_vm(vm, || {
        serde_json::from_slice(&bytes).map_err(st::Error::from)
    })?;

    vm.managed_push(value_ptr)?;
    Ok(())
}

fn from_string(vm: &mut Vm, args: usize) -> Result<(), VmError> {
    if args != 1 {
        return Err(VmError::ArgumentCountMismatch {
            actual: args,
            expected: 1,
        });
    }

    let bytes = vm.pop_decode::<String>()?;

    let value_ptr: ValuePtr = st::tls::inject_vm(vm, || {
        serde_json::from_str(&bytes).map_err(st::Error::from)
    })?;

    vm.managed_push(value_ptr)?;
    Ok(())
}

/// Get the module for the bytes package.
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["json"]);
    module.raw_fn("from_bytes", from_bytes)?;
    module.raw_fn("from_string", from_string)?;
    Ok(module)
}
