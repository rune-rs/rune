use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr;

use crate::{ContextError, InternalContextError, InternalVmError, Stack, VmError};

pub(crate) type InternalModule = Option<rune::Module>;

/// The signature of a custom function.
///
/// Where `stack` is the stack being interacted with and `count` are the number
/// of arguments passed in.
pub type Function = extern "C" fn(stack: *mut Stack, count: usize, *mut VmError);

/// A module with custom functions and the like.
#[repr(C)]
pub struct Module {
    repr: [u8; 408],
}

test_size!(Module, InternalModule);

/// Construct a new context.
#[no_mangle]
pub extern "C" fn rune_module_new() -> Module {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(Some(rune::Module::new())) }
}

/// Free the given module.
///
/// # Safety
///
/// The `module` argument must have been allocated with [rune_module_new].
#[no_mangle]
pub unsafe extern "C" fn rune_module_free(module: *mut Module) {
    let _ = ptr::replace(module as *mut InternalModule, None);
}

/// Register a toplevel function to the module.
///
/// Returns `false` if the module is freed or the name is not valid UTF-8.
///
/// # Safety
///
/// The `module` argument must have been allocated with [rune_module_new] and
/// `name` must be a NULL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rune_module_function(
    module: *mut Module,
    name: *const c_char,
    function: Function,
    error: *mut ContextError,
) -> bool {
    let name = CStr::from_ptr(name);

    let module = match &mut *(module as *mut InternalModule) {
        Some(module) => module,
        None => return false,
    };

    let name = match name.to_str() {
        Ok(name) => name,
        Err(..) => return false,
    };

    let result = module.raw_fn(&[name], move |stack, count| {
        let stack = stack as *mut _ as *mut Stack;
        let mut error: VmError = mem::transmute(InternalVmError::None);
        function(stack, count, &mut error);

        if let Some(error) = (&mut *(&mut error as *mut VmError).cast::<InternalVmError>()).take() {
            return Err(error);
        }

        Ok(())
    });

    if let Err(e) = result {
        let _ = ptr::replace(error as *mut InternalContextError, Some(e));
        return false;
    }

    true
}
