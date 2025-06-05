use core::fmt;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

use crate::alloc;
use crate::alloc::alloc::Global;
use crate::alloc::clone::TryClone;
use crate::alloc::sync::Arc;
use crate::runtime::{Address, Memory, Output, VmError};

/// The vtable for a function handler.
struct FunctionHandlerVTable {
    call: unsafe fn(ptr: *const (), &mut dyn Memory, Address, usize, Output) -> Result<(), VmError>,
    drop: unsafe fn(ptr: *const ()),
    clone: unsafe fn(ptr: *const ()) -> *const (),
}

/// A raw function handler in the rune virtual machine.
pub struct FunctionHandler {
    ptr: NonNull<()>,
    vtable: &'static FunctionHandlerVTable,
}

impl FunctionHandler {
    #[inline]
    pub(crate) fn new<F>(f: F) -> alloc::Result<Self>
    where
        F: Fn(&mut dyn Memory, Address, usize, Output) -> Result<(), VmError>
            + Send
            + Sync
            + 'static,
    {
        let arc = Arc::try_new(f)?;
        let (ptr, Global) = Arc::into_raw_with_allocator(arc);
        let ptr = unsafe { NonNull::new_unchecked(ptr.cast_mut().cast()) };
        let vtable = &FunctionHandlerVTable {
            call: call_impl::<F>,
            drop: drop_impl::<F>,
            clone: clone_impl::<F>,
        };

        Ok(Self { ptr, vtable })
    }

    /// Call the function handler through the raw type-erased API.
    #[inline]
    pub fn call(
        &self,
        memory: &mut dyn Memory,
        addr: Address,
        count: usize,
        out: Output,
    ) -> Result<(), VmError> {
        // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
        unsafe { (self.vtable.call)(self.ptr.as_ptr().cast_const(), memory, addr, count, out) }
    }
}

unsafe impl Send for FunctionHandler {}
unsafe impl Sync for FunctionHandler {}

impl Drop for FunctionHandler {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
        unsafe { (self.vtable.drop)(self.ptr.as_ptr().cast_const()) }
    }
}

fn call_impl<F>(
    ptr: *const (),
    memory: &mut dyn Memory,
    addr: Address,
    count: usize,
    out: Output,
) -> Result<(), VmError>
where
    F: Fn(&mut dyn Memory, Address, usize, Output) -> Result<(), VmError> + Send + Sync + 'static,
{
    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
    unsafe { (*ptr.cast::<F>())(memory, addr, count, out) }
}

fn clone_impl<F>(ptr: *const ()) -> *const () {
    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
    unsafe {
        let ptr = ptr.cast::<F>();
        // Prevent the constructed Arc from being dropped, which would decrease
        // its reference count.
        let arc = ManuallyDrop::new(Arc::<F>::from_raw_in(ptr, Global));
        let arc = (*arc).clone();
        let (ptr, Global) = Arc::into_raw_with_allocator(arc);
        ptr.cast()
    }
}

fn drop_impl<F>(ptr: *const ()) {
    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
    unsafe {
        let ptr = ptr.cast::<F>();
        drop(Arc::<F, Global>::from_raw_in(ptr, Global));
    }
}

impl Clone for FunctionHandler {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: The pointer is valid and the vtable is static.
        let ptr = unsafe {
            NonNull::new_unchecked((self.vtable.clone)(self.ptr.as_ptr().cast_const()).cast_mut())
        };

        Self {
            ptr,
            vtable: self.vtable,
        }
    }
}

impl TryClone for FunctionHandler {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }
}

impl fmt::Pointer for FunctionHandler {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}
