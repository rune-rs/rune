use core::ptr::NonNull;

use crate::hash::Hash;
use crate::runtime::VmResult;
use crate::Diagnostics;

pub trait VmDiagnostics {
    fn function_used(&mut self, hash: Hash, at: usize) -> VmResult<()>;

    fn vtable(&self) -> &'static VmDiagnosticsObjVtable;
}

impl VmDiagnostics for Diagnostics {
    fn function_used(&mut self, hash: Hash, at: usize) -> VmResult<()> {
        vm_try!(self.runtime_used_deprecated(at, hash));
        VmResult::Ok(())
    }

    fn vtable(&self) -> &'static VmDiagnosticsObjVtable {
        fn function_used_impl<T>(ptr: NonNull<()>, hash: Hash, at: usize) -> VmResult<()>
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
    function_used: unsafe fn(NonNull<()>, hash: Hash, at: usize) -> VmResult<()>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmDiagnosticsObj {
    ptr: NonNull<()>,
    vtable: &'static VmDiagnosticsObjVtable,
}

impl VmDiagnosticsObj {
    pub(crate) fn new(trait_obj: &mut dyn VmDiagnostics) -> Self {
        let vtable = trait_obj.vtable();

        Self {
            ptr: unsafe { NonNull::new_unchecked(trait_obj as *mut _ as *mut ()) },
            vtable,
        }
    }

    pub(crate) fn function_used(&mut self, hash: Hash, at: usize) -> VmResult<()> {
        unsafe { (self.vtable.function_used)(self.ptr, hash, at) }
    }
}
