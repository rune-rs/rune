use std::{mem, ptr, sync::Arc};

/// The internal runtime context.
pub(crate) type InternalRuntimeContext = Option<Arc<rune::runtime::RuntimeContext>>;

/// A runtime context.
#[repr(C)]
pub struct RuntimeContext {
    repr: [u8; 8],
}

test_size!(RuntimeContext, InternalRuntimeContext);

/// Allocate an empty runtime context.
#[no_mangle]
pub extern "C" fn rune_runtime_context_new() -> RuntimeContext {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(InternalRuntimeContext::None) }
}

/// Free the given runtime context.
///
/// This is a reference counted object. If the reference counts goes to 0, the
/// underlying object is freed.
///
/// # Safety
///
/// Function must be called with a `runtime` argument that has been allocated by
/// [rune_runtime_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_runtime_context_free(runtime: *mut RuntimeContext) {
    let _ = ptr::replace(runtime as *mut InternalRuntimeContext, None);
}

/// Clone the given runtime context and return a new reference.
///
/// # Safety
///
/// Function must be called with a `runtime` argument that has been allocated by
/// [rune_runtime_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_runtime_context_clone(
    runtime: *const RuntimeContext,
) -> RuntimeContext {
    let runtime = &*(runtime as *const InternalRuntimeContext);
    mem::transmute(runtime.clone())
}
