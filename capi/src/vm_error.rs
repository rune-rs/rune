use std::{mem, ptr};

use crate::{InternalSources, InternalStandardStream, Sources, StandardStream, StaticType, Value};

pub(crate) type InternalVmError = Option<rune::runtime::VmError>;

/// An error that can be raised by a virtual machine.
///
/// This must be declared with [rune_vm_error_new] and must be freed with
/// [rune_vm_error_free].
///
/// \code{.c}
/// int main() {
///     rune_vm_error error = rune_vm_error_new();
///
///     // ...
///
///     rune_vm_error_free(&error);
/// }
/// \endcode
#[repr(C)]
pub struct VmError {
    repr: [u8; 8],
}

test_size!(VmError, InternalVmError);

/// Construct an empty [VmError].
#[no_mangle]
pub extern "C" fn rune_vm_error_new() -> VmError {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(InternalVmError::None) }
}

/// Free the given virtual machine error.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_vm_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_error_free(error: *mut VmError) {
    let _ = ptr::replace(error as *mut InternalVmError, None);
}

/// Emit diagnostics to the given stream if the error is set. If the error is
/// not set nothing will be emitted.
///
/// TODO: propagate I/O errors somehow.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_vm_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_error_emit(
    error: *const VmError,
    stream: *mut StandardStream,
    sources: *const Sources,
) -> bool {
    let error = &*(error as *const InternalVmError);
    let stream = &mut *(stream as *mut InternalStandardStream);
    let sources = &*(sources as *const InternalSources);

    if let (Some(error), Some(stream)) = (error, stream) {
        error.emit(stream, sources).is_ok()
    } else {
        false
    }
}

/// Set the given error to report a bad argument count error where the `actual`
/// number of arguments were provided instead of `expected`.
///
/// This will replace any errors already reported.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_vm_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_error_bad_argument_count(
    error: *mut VmError,
    actual: usize,
    expected: usize,
) {
    ptr::replace(
        error as *mut InternalVmError,
        Some(rune::runtime::VmError::bad_argument_count(actual, expected)),
    );
}

/// Set the given error to report a bad argument at the given position, which
/// did not have the `expected` type.
///
/// This will replace any errors already reported.
///
/// # Safety
///
/// Must be called with an error that has been allocated with
/// [rune_vm_error_new].
#[no_mangle]
pub unsafe extern "C" fn rune_vm_error_bad_argument_at(
    error: *mut VmError,
    arg: usize,
    actual: *const Value,
    expected: StaticType,
) {
    let actual = &*(actual as *const rune::Value);
    let expected = &*(expected.inner as *const rune::runtime::StaticType);
    let expected = rune::runtime::TypeInfo::StaticType(expected);

    let e = match rune::runtime::VmError::ffi_bad_argument_at(arg, actual, expected) {
        Ok(e) => e,
        Err(e) => e,
    };

    let _ = ptr::replace(error as *mut InternalVmError, Some(e));
}
