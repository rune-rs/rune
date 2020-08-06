//! Utilities for storing and accessing the virtual machine from thread-local
//! storage.
//!
//! **Warning:** This is potentially very unsafe, and maybe even unsound.
//!
//! The serde implementation of `VirtualPtr` relies on being called inside of
//! [with_vm].

use crate::unit::CompilationUnit;
use crate::vm::Vm;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

thread_local!(static VM: RefCell<Option<Tls>> = RefCell::new(None));

struct Tls {
    vm: NonNull<Vm>,
    unit: *const CompilationUnit,
}

/// Guard that restored the old VM in the threadlocal when dropped.
struct Guard<'a>(&'a RefCell<Option<Tls>>, Option<Tls>);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(vm) = self.1.take() {
            *self.0.borrow_mut() = Some(vm);
        }
    }
}

/// Inject the vm into TLS while running the given closure.
pub fn inject_vm<F, O>(vm: &mut Vm, unit: &CompilationUnit, f: F) -> O
where
    F: FnOnce() -> O,
{
    let vm = unsafe { NonNull::new_unchecked(vm) };
    let tls = Tls { vm, unit };

    VM.with(|storage| {
        let old_tls = storage.borrow_mut().replace(tls);
        let _guard = Guard(&storage, old_tls);
        f()
    })
}

/// Run the given closure with access to the vm.
pub fn with_vm<F, O>(f: F) -> O
where
    F: FnOnce(&mut Vm, &CompilationUnit) -> O,
{
    VM.with(|storage| {
        let mut b = storage.borrow_mut();
        let b = b.as_mut().expect("vm must be available");
        unsafe { f(b.vm.as_mut(), &*b.unit) }
    })
}

/// A future which wraps polls to have access to the TLS virtual machine.
pub struct InjectVm<'vm, T> {
    vm: &'vm mut Vm,
    unit: &'vm CompilationUnit,
    future: T,
}

impl<'vm, T> InjectVm<'vm, T> {
    /// Construct a future that gives the called future thread-local access to
    /// the virtual machine.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `InjectVm` is correctly pinned w.r.t. its inner
    /// future.
    pub unsafe fn new(vm: &'vm mut Vm, unit: &'vm CompilationUnit, future: T) -> Self {
        Self { vm, unit, future }
    }
}

impl<'vm, T> Future for InjectVm<'vm, T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: This future can only be constructed unsafely, and relies on
        // being embedded into an already pinned future and called directly.
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let future = Pin::new_unchecked(&mut this.future);
            inject_vm(this.vm, this.unit, || future.poll(cx))
        }
    }
}
