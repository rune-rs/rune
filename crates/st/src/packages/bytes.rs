//! The bytes package, providing access to the bytes type.

use crate::{Functions, RegisterError};
use std::fmt;

#[derive(Clone)]
struct Bytes {
    bytes: Vec<u8>,
}

impl fmt::Debug for Bytes {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.bytes, fmt)
    }
}

impl Bytes {
    /// Construct a new bytes container.
    fn new() -> Self {
        Bytes { bytes: Vec::new() }
    }

    /// Do something with the bytes.
    fn extend(&mut self, other: &Self) {
        self.bytes.extend(other.bytes.iter().copied());
    }

    /// Do something with the bytes.
    fn extend_str(&mut self, s: &str) {
        self.bytes.extend(s.as_bytes());
    }

    /// Get the length of the bytes collection.
    fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Get the bytes collection.
    fn clear(&mut self) {
        self.bytes.clear();
    }
}

decl_external!(Bytes);

/// Install the bytes package.
pub fn install(functions: &mut Functions) -> Result<(), RegisterError> {
    let global_module = functions.global_module_mut();
    global_module.global_fn("bytes", Bytes::new)?;
    global_module.instance_fn("extend", Bytes::extend)?;
    global_module.instance_fn("extend_str", Bytes::extend_str)?;
    global_module.instance_fn("len", Bytes::len)?;
    global_module.instance_fn("clear", Bytes::clear)?;
    global_module.instance_fn("clone", Bytes::clone)?;
    Ok(())
}
