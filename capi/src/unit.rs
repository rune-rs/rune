use std::mem;
use std::ptr;
use std::sync::Arc;

pub(crate) type InternalUnit = Option<Arc<rune::Unit>>;

/// A rune source file.
#[repr(C)]
pub struct Unit {
    repr: [u8; 8],
}

test_size!(Unit, InternalUnit);

/// Construct a new empty unit handle.
#[no_mangle]
pub extern "C" fn rune_unit_new() -> Unit {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(InternalUnit::None) }
}

/// Free a unit. Calling this multiple times on the same handle is allowed.
///
/// This is a reference counted object. If the reference counts goes to 0, the
/// underlying object is freed.
///
/// # Safety
///
/// The `unit` argument must have been allocated with [rune_unit_new].
#[no_mangle]
pub unsafe extern "C" fn rune_unit_free(unit: *mut Unit) {
    let _ = ptr::replace(unit as *mut InternalUnit, None);
}

/// Clone the given unit and return a new handle. Cloning increases the
/// reference count of the unit by one.
///
/// # Safety
///
/// The `unit` argument must have been allocated with [rune_unit_new].
#[no_mangle]
pub unsafe extern "C" fn rune_unit_clone(unit: *const Unit) -> Unit {
    let unit = &*(unit as *const InternalUnit);
    mem::transmute(unit.clone())
}
