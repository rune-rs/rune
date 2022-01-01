use std::{mem, ptr};

use crate::{InternalSource, Source};

/// Internal sources repr.
pub(crate) type InternalSources = rune::Sources;

/// A collection of sources.
#[repr(C)]
pub struct Sources {
    repr: [u8; 24],
}

test_size!(Sources, InternalSources);

/// Construct a new [rn_sources] object.
#[no_mangle]
pub extern "C" fn rune_sources_new() -> Sources {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(rune::Sources::new()) }
}

/// Insert a source to be compiled. Once inserted, they are part of sources
/// collection and do not need to be freed.
///
/// Returns `true` if the source was successfully inserted. Otherwise it means
/// that the provided source was empty.
///
/// # Safety
///
/// Must be called with a `sources` collection allocated with
/// [rune_sources_new].
#[no_mangle]
pub unsafe extern "C" fn rune_sources_insert(sources: *mut Sources, source: *mut Source) -> bool {
    let sources = &mut *(sources as *mut InternalSources);

    if let Some(source) = ptr::replace(source as *mut InternalSource, None) {
        sources.insert(source);
        true
    } else {
        false
    }
}

/// Free a sources collection. After it's been freed the collection is no longer
/// valid.
///
/// # Safety
///
/// Must be called with a `sources` collection allocated with
/// [rune_sources_new].
#[no_mangle]
pub unsafe extern "C" fn rune_sources_free(sources: *mut Sources) {
    ptr::drop_in_place(sources as *mut InternalSources);
}
