//! Utilities for storing and accessing the virtual machine from thread-local
//! storage.
//!
//! **Warning:** This is potentially very unsafe, and maybe even unsound.
//!
//! The serde implementation of `VirtualPtr` relies on being called inside of
//! [with_vm].

use crate::vm::Vm;
use std::cell::RefCell;

thread_local!(static VM: RefCell<*mut Vm> = RefCell::new(std::ptr::null_mut()));

/// Inject the vm into TLS while running the given closure.
pub fn inject_vm<F, O>(vm: &mut Vm, f: F) -> O
where
    F: FnOnce() -> O,
{
    let vm = vm as *mut _;

    VM.with(|storage| {
        *storage.borrow_mut() = vm;
    });

    let result = f();

    VM.with(|storage| {
        *storage.borrow_mut() = std::ptr::null_mut();
    });

    result
}

/// Run the given closure with access to the vm.
pub fn with_vm<F, O>(f: F) -> O
where
    F: FnOnce(&mut Vm) -> O,
{
    VM.with(|storage| {
        let mut b = storage.borrow_mut();
        assert!(!b.is_null(), "virtual machine is not available");
        f(unsafe { &mut **b })
    })
}
