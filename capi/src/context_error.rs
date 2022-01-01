use std::mem;
use std::ptr;

use crate::{InternalStandardStream, StandardStream};

pub(crate) type InternalContextError = Option<rune::compile::ContextError>;

/// An error that can be raised by a virtual machine.
///
/// This must be declared with [rune_context_error_new] and must be freed with
/// [rune_context_error_free].
///
/// \code{.c}
/// int main() {
///     rune_context_error error = rune_context_error_new();
///
///     // ...
///
///     rune_context_error_free(&error);
/// }
/// \endcode
#[repr(C)]
pub struct ContextError {
    repr: [u8; 152],
}

test_size!(ContextError, InternalContextError);

/// Construct an empty [ContextError].
#[no_mangle]
pub extern "C" fn rune_context_error_new() -> ContextError {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(InternalContextError::None) }
}

/// Free the given context error.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_context_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_context_error_free(error: *mut ContextError) {
    let _ = ptr::replace(error as *mut InternalContextError, None);
}

/// Emit diagnostics to the given stream if the error is set. If the error is
/// not set nothing will be emitted.
///
/// TODO: propagate I/O errors somehow.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_context_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_context_error_emit(
    error: *const ContextError,
    stream: *mut StandardStream,
) -> bool {
    use std::io::Write;

    let error = &*(error as *const InternalContextError);
    let stream = &mut *(stream as *mut InternalStandardStream);

    if let (Some(error), Some(stream)) = (error, stream) {
        writeln!(stream, "{}", error).is_ok()
    } else {
        false
    }
}
