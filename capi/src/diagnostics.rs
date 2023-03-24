use std::mem;
use std::ptr;

use crate::{InternalSources, InternalStandardStream, Sources, StandardStream};

/// Build diagnostics internal.
pub(crate) type InternalDiagnostics = Option<rune::Diagnostics>;

/// Build diagnostics.
#[repr(C)]
pub struct Diagnostics {
    repr: [u8; 32],
}

test_size!(Diagnostics, InternalDiagnostics);

/// Construct a new build diagnostics instance.
///
/// Used with [rn_build_diagnostics][crate:rn_build_diagnostics].
#[no_mangle]
pub extern "C" fn rune_diagnostics_new() -> Diagnostics {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(Some(rune::Diagnostics::new())) }
}

/// Free a build diagnostics instance.
///
/// # Safety
///
/// Function must be called with a diagnostics object allocated by
/// [rune_diagnostics_new].
#[no_mangle]
pub unsafe extern "C" fn rune_diagnostics_free(diagnostics: *mut Diagnostics) {
    let _ = ptr::replace(diagnostics as *mut InternalDiagnostics, None);
}

/// Test if diagnostics is empty. Will do nothing if the diagnostics object is
/// not present.
///
/// # Safety
///
/// Function must be called with a diagnostics object allocated by
/// [rune_diagnostics_new].
#[no_mangle]
pub unsafe extern "C" fn rune_diagnostics_is_empty(diagnostics: *const Diagnostics) -> bool {
    if let Some(diagnostics) = &*(diagnostics as *mut InternalDiagnostics) {
        diagnostics.is_empty()
    } else {
        true
    }
}

/// Emit diagnostics to the given stream.
///
/// TODO: propagate I/O errors somehow.
///
/// # Safety
///
/// Function must be called with a diagnostics object allocated by
/// [rune_diagnostics_new] and a valid `stream` and `sources` argument.
#[no_mangle]
pub unsafe extern "C" fn rune_diagnostics_emit(
    diagnostics: *const Diagnostics,
    stream: *mut StandardStream,
    sources: *const Sources,
) -> bool {
    let stream = &mut *(stream as *mut InternalStandardStream);
    let diagnostics = &*(diagnostics as *mut InternalDiagnostics);

    if let (Some(diagnostics), Some(stream)) = (diagnostics, stream) {
        let sources = &*(sources as *const InternalSources);
        diagnostics.emit(stream, sources).is_ok()
    } else {
        false
    }
}
