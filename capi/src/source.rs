use std::ffi::CStr;
use std::os::raw::c_char;
use std::{mem, ptr};

/// A rune source file.
pub(crate) type InternalSource = Option<rune::Source>;

/// A rune source file.
#[repr(C)]
pub struct Source {
    repr: [u8; 72],
}

test_size!(Source, InternalSource);

/// Construct a compile source.
///
/// Returns an empty source if the name or the source is not valid UTF-8.
///
/// # Safety
///
/// Must be called a `name` and `source` argument that points to valid
/// NULL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn rune_source_new(name: *const c_char, source: *const c_char) -> Source {
    let name = CStr::from_ptr(name);
    let source = CStr::from_ptr(source);

    let name = match name.to_str() {
        Ok(name) => name,
        Err(..) => return mem::transmute(InternalSource::None),
    };

    let source = match source.to_str() {
        Ok(source) => source,
        Err(..) => return mem::transmute(InternalSource::None),
    };

    let source = rune::Source::new(name, source);
    mem::transmute(Some(source))
}

/// Free a compile source. Does nothing if it has already been freed.
///
/// # Safety
///
/// Must be called with a `source` that has been allocation with
/// [rune_source_new].
#[no_mangle]
pub unsafe extern "C" fn rune_source_free(source: *mut Source) {
    let _ = ptr::replace(source as *mut InternalSource, None);
}
