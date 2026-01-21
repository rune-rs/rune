use core::cell::RefCell;

use crate::alloc::{self, String};

/// Parse binary from ASCII bytes.
pub(crate) fn from_ascii_binary(bytes: &[u8]) -> Option<u64> {
    let mut v = 0u64;

    for c in bytes {
        let a = match c {
            b'0'..=b'1' => (c - b'0') as u64,
            b'_' => continue,
            _ => return None,
        };

        v = v.checked_mul(2)?.checked_add(a)?;
    }

    Some(v)
}

/// Parse octal from ASCII bytes.
pub(crate) fn from_ascii_octal(bytes: &[u8]) -> Option<u64> {
    let mut v = 0u64;

    for c in bytes {
        let a = match c {
            b'0'..=b'7' => (c - b'0') as u64,
            b'_' => continue,
            _ => return None,
        };

        v = v.checked_mul(8)?.checked_add(a)?;
    }

    Some(v)
}

/// Parse a hexadecimal number from ASCII bytes.
pub(crate) fn from_ascii_hex(bytes: &[u8]) -> Option<u64> {
    let mut v = 0u64;

    for c in bytes {
        let a = match c {
            b'0'..=b'9' => (c - b'0') as u64,
            b'a'..=b'f' => (c - b'a' + 10) as u64,
            b'A'..=b'F' => (c - b'A' + 10) as u64,
            b'_' => continue,
            _ => return None,
        };

        v = v.checked_mul(16)?.checked_add(a)?;
    }

    Some(v)
}

/// Parse a decimal integer from ASCII bytes.
pub(crate) fn from_ascii_decimal(bytes: &[u8]) -> Option<u64> {
    let mut v = 0u64;

    for c in bytes {
        let a = match c {
            b'0'..=b'9' => (c - b'0') as u64,
            b'_' => continue,
            _ => return None,
        };

        v = v.checked_mul(10)?.checked_add(a)?;
    }

    Some(v)
}

/// Errors when parsing a float.
pub(crate) enum FromFloatError {
    ScratchInUse,
    Alloc(alloc::Error),
    Error,
}

/// Parse a float.
///
/// Because the literal float might contain underscores, we have to use a
/// scratch buffer when parsing it out.
pub(crate) fn from_float(scratch: &RefCell<String>, string: &str) -> Result<f64, FromFloatError> {
    let string = string.trim_matches('_');

    if !string.contains('_') {
        return string.parse().map_err(|_| FromFloatError::Error);
    }

    let Ok(mut scratch) = scratch.try_borrow_mut() else {
        return Err(FromFloatError::ScratchInUse);
    };

    scratch.clear();
    scratch
        .try_push_str(string)
        .map_err(FromFloatError::Alloc)?;
    scratch.retain(|c| c != '_');
    scratch.parse().map_err(|_| FromFloatError::Error)
}
