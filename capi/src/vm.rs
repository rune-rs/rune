use std::mem;
use std::ptr;

use crate::{
    Hash, InternalRuntimeContext, InternalUnit, InternalVmError, RuntimeContext, Stack, Unit,
    Value, VmError,
};

/// Internal virtual machine representation.
pub(crate) type VmInternal = Option<rune::Vm>;

/// A virtual machine.
#[repr(C)]
pub struct Vm {
    repr: [u8; 80],
}

test_size!(Vm, VmInternal);

/// Allocate space for a virtual machine.
#[no_mangle]
pub extern "C" fn rune_vm_new() -> Vm {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(VmInternal::None) }
}

/// Set up new virtual machine and assign it to `vm`.
///
/// This takes ownership of the passed in `unit` and `runtime`. If either the
/// `runtime` or `unit` is not set this function will return `false`.
///
/// # Safety
///
/// Must be called with a `vm` that has been allocated with [rune_vm_new] and a
/// valid `runtime` and `unit` argument.
#[no_mangle]
pub unsafe extern "C" fn rune_vm_setup(
    vm: *mut Vm,
    runtime: *mut RuntimeContext,
    unit: *mut Unit,
) -> bool {
    let runtime = ptr::replace(runtime as *mut InternalRuntimeContext, None);
    let unit = ptr::replace(unit as *mut InternalUnit, None);

    let (runtime, unit) = match (runtime, unit) {
        (Some(runtime), Some(unit)) => (runtime, unit),
        _ => return false,
    };

    let _ = ptr::replace(
        vm as *mut VmInternal,
        VmInternal::Some(rune::Vm::new(runtime, unit)),
    );

    true
}

/// Run the virtual machine to completion.
///
/// This will replace `value`, freeing any old value which is already in place.
///
/// Returns `true` if the virtual machine was successfully run to completion.
///
/// # Safety
///
/// Must be called with a `vm` that has been allocated with [rune_vm_new] and a
/// valid `value` and `error` argument.
#[no_mangle]
pub unsafe extern "C" fn rune_vm_complete(
    vm: *mut Vm,
    value: *mut Value,
    error: *mut VmError,
) -> bool {
    let vm = match &mut *(vm as *mut VmInternal) {
        Some(vm) => vm,
        None => return false,
    };

    match vm.complete() {
        Ok(v) => {
            // Replace to ensure old value is freed.
            let _ = ptr::replace(value as *mut rune::Value, v);
            true
        }
        Err(e) => {
            let _ = ptr::replace(error as *mut InternalVmError, Some(e));
            false
        }
    }
}

/// Set the entrypoint to the given hash in the virtual machine.
///
/// # Safety
///
/// Must be called with a `vm` that has been allocated with [rune_vm_new] and a
/// valid `error` argument.
#[no_mangle]
pub unsafe extern "C" fn rune_vm_set_entrypoint(
    vm: *mut Vm,
    hash: Hash,
    args: usize,
    error: *mut VmError,
) -> bool {
    let vm = match &mut *(vm as *mut VmInternal) {
        Some(vm) => vm,
        None => {
            let _ = ptr::replace(
                error as *mut InternalVmError,
                Some(rune::runtime::VmError::ffi_missing_vm()),
            );
            return false;
        }
    };

    let hash: rune::Hash = mem::transmute(hash);

    match vm.ffi_set_entrypoint(hash, args) {
        Ok(()) => true,
        Err(e) => {
            // Replace the existing error.
            let _ = ptr::replace(error as *mut InternalVmError, Some(e));
            false
        }
    }
}

/// Get the stack associated with the virtual machine. If `vm` is not set returns NULL.
///
/// # Safety
///
/// Must be called with a `vm` that has been allocated with [rune_vm_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_stack_mut(vm: *mut Vm) -> *mut Stack {
    let vm = match &mut *(vm as *mut VmInternal) {
        Some(stack) => stack,
        None => return ptr::null_mut(),
    };

    vm.stack_mut() as *mut _ as *mut Stack
}

/// Free a virtual machine.
///
/// # Safety
///
/// Must be called with a `vm` that has been allocated with [rune_vm_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_free(vm: *mut Vm) {
    let _ = ptr::replace(vm as *mut VmInternal, None);
}
