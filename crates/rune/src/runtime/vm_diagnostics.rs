use core::ptr::NonNull;

use crate::hash::Hash;
use crate::runtime::VmError;
use crate::Diagnostics;

/// A trait for runtime diagnostics in the virtual machine.
pub trait VmDiagnostics {
    /// Mark that a function has been used.
    fn function_used(&mut self, hash: Hash, at: usize) -> Result<(), VmError>;

    /// Returns the vtable for this diagnostics object.
    #[doc(hidden)]
    fn vtable(&self) -> &'static VmDiagnosticsObjVtable;
}

impl VmDiagnostics for Diagnostics {
    #[inline]
    fn function_used(&mut self, hash: Hash, at: usize) -> Result<(), VmError> {
        self.runtime_used_deprecated(at, hash)?;
        Ok(())
    }

    #[inline]
    fn vtable(&self) -> &'static VmDiagnosticsObjVtable {
        fn function_used_impl<T>(ptr: NonNull<()>, hash: Hash, at: usize) -> Result<(), VmError>
        where
            T: VmDiagnostics,
        {
            unsafe { VmDiagnostics::function_used(ptr.cast::<T>().as_mut(), hash, at) }
        }

        &VmDiagnosticsObjVtable {
            function_used: function_used_impl::<Self>,
        }
    }
}

#[derive(Debug)]
pub struct VmDiagnosticsObjVtable {
    function_used: unsafe fn(NonNull<()>, hash: Hash, at: usize) -> Result<(), VmError>,
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
    pub(crate) fn function_used(&mut self, hash: Hash, at: usize) -> Result<(), VmError> {
        unsafe { (self.vtable.function_used)(self.ptr, hash, at) }
    }
}
