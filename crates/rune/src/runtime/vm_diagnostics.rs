use core::ptr::NonNull;

use crate::hash::Hash;
use crate::runtime::{RuntimeContext, VmError};
use crate::Diagnostics;

/// A trait for runtime diagnostics in the virtual machine.
pub trait VmDiagnostics {
    /// Mark that a function has been used.
    ///
    /// This is called for every native function which is being called, so any
    /// filtering of interesting hashes has to be performed here. The `context`
    /// in which the function was called is provided for that purpose, see for
    /// example [`RuntimeContext::deprecation`].
    fn function_used(
        &mut self,
        context: &RuntimeContext,
        hash: Hash,
        at: usize,
    ) -> Result<(), VmError>;

    /// Returns the vtable for this diagnostics object.
    #[doc(hidden)]
    fn vtable(&self) -> &'static VmDiagnosticsObjVtable;
}

impl VmDiagnostics for Diagnostics {
    #[inline]
    fn function_used(
        &mut self,
        context: &RuntimeContext,
        hash: Hash,
        at: usize,
    ) -> Result<(), VmError> {
        // Only functions which have actually been marked as deprecated are of
        // interest, since recording every function call would be prohibitively
        // expensive.
        if context.deprecation(&hash).is_some() {
            self.runtime_used_deprecated(at, hash)?;
        }

        Ok(())
    }

    #[inline]
    fn vtable(&self) -> &'static VmDiagnosticsObjVtable {
        fn function_used_impl<T>(
            ptr: NonNull<()>,
            context: &RuntimeContext,
            hash: Hash,
            at: usize,
        ) -> Result<(), VmError>
        where
            T: VmDiagnostics,
        {
            unsafe { VmDiagnostics::function_used(ptr.cast::<T>().as_mut(), context, hash, at) }
        }

        &VmDiagnosticsObjVtable {
            function_used: function_used_impl::<Self>,
        }
    }
}

#[derive(Debug)]
pub struct VmDiagnosticsObjVtable {
    function_used: unsafe fn(
        NonNull<()>,
        context: &RuntimeContext,
        hash: Hash,
        at: usize,
    ) -> Result<(), VmError>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmDiagnosticsObj {
    ptr: NonNull<()>,
    vtable: &'static VmDiagnosticsObjVtable,
}

impl VmDiagnosticsObj {
    #[inline]
    pub(crate) fn new(trait_obj: &mut dyn VmDiagnostics) -> Self {
        let vtable = trait_obj.vtable();

        Self {
            ptr: unsafe { NonNull::new_unchecked(trait_obj as *mut _ as *mut ()) },
            vtable,
        }
    }

    #[inline]
    pub(crate) fn function_used(
        &mut self,
        context: &RuntimeContext,
        hash: Hash,
        at: usize,
    ) -> Result<(), VmError> {
        unsafe { (self.vtable.function_used)(self.ptr, context, hash, at) }
    }
}
