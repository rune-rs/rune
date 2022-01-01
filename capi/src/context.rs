use std::mem;
use std::ptr;
use std::sync::Arc;

use crate::{
    ContextError, InternalContextError, InternalModule, InternalRuntimeContext, Module,
    RuntimeContext,
};

pub(crate) type InternalContext = Option<rune::Context>;

/// A context.
#[repr(C)]
pub struct Context {
    repr: [u8; 656],
}

test_size!(Context, InternalContext);

/// Construct a new context.
#[no_mangle]
pub extern "C" fn rune_context_new() -> Context {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(Some(rune::Context::new())) }
}

/// Free a context. After it's been freed the context is no longer valid.
///
/// # Safety
///
/// Must be called with a context allocated through [rune_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_context_free(context: *mut Context) {
    let _ = ptr::replace(context as *mut InternalContext, None);
}

/// Install the given module into the current context.
///
/// Returns `false` if either context or `module` is not present or if
/// installation fails.
///
/// # Safety
///
/// The current `context` must have been allocated with [rune_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_context_install(
    context: *mut Context,
    module: *const Module,
    error: *mut ContextError,
) -> bool {
    let context = &mut *(context as *mut InternalContext);
    let module = &*(module as *const InternalModule);

    let (context, module) = match (context, module) {
        (Some(context), Some(module)) => (context, module),
        _ => return false,
    };

    if let Err(e) = context.install(module) {
        let _ = ptr::replace(error as *mut InternalContextError, Some(e));
        false
    } else {
        true
    }
}

/// Construct a runtime context from the current context.
///
/// # Safety
///
/// Function must be called with a `context` object allocated by
/// [rune_context_new] and a valid `runtime` argument allocated with
/// [rune_runtime_context_new][crate::rune_runtime_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_context_runtime(
    context: *const Context,
    runtime: *mut RuntimeContext,
) -> bool {
    if let Some(context) = &*(context as *const InternalContext) {
        let new = Arc::new(context.runtime());
        let _ = ptr::replace(runtime as *mut InternalRuntimeContext, Some(new));
        true
    } else {
        false
    }
}
