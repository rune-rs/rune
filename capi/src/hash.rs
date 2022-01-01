use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;

/// The type hash of an integer.
pub const RUNE_INTEGER_TYPE_HASH: Hash = 0xbb378867da3981e2;
/// The type hash of a boolean.
pub const RUNE_BOOL_TYPE_HASH: Hash = 0xbe6bff4422d0c759;

/// An opaque hash.
pub type Hash = u64;

test_size!(Hash, rune::Hash);

/// Construct the empty hash.
#[no_mangle]
pub extern "C" fn rune_hash_empty() -> Hash {
    unsafe { mem::transmute(rune::Hash::EMPTY) }
}

/// Generate a hash corresponding to the given name.
///
/// Returns an empty hash that can be tested with [rn_hash_is_empty].
///
/// # Safety
///
/// Function must be called with a non-NULL `name` argument.
#[no_mangle]
pub unsafe extern "C" fn rune_hash_name(name: *const c_char) -> Hash {
    let hash = if let Ok(string) = CStr::from_ptr(name).to_str() {
        rune::Hash::type_hash(&[string])
    } else {
        rune::Hash::EMPTY
    };

    mem::transmute(hash)
}

/// Test if the hash is empty.
#[no_mangle]
pub extern "C" fn rune_hash_is_empty(hash: Hash) -> bool {
    // Safety: Hash can inhabit all possible bit patterns.
    unsafe { mem::transmute::<_, rune::Hash>(hash) == rune::Hash::EMPTY }
}
