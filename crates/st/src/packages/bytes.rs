//! The bytes package, providing access to the bytes type.

use crate::{Functions, RegisterError};
use std::fmt;

/// A bytes container.
#[derive(Clone)]
pub struct Bytes {
    bytes: Vec<u8>,
}

impl fmt::Debug for Bytes {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.bytes, fmt)
    }
}

impl Bytes {
    /// Construct from a byte array.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Construct a new bytes container.
    fn new() -> Self {
        Bytes { bytes: Vec::new() }
    }

    /// Construct a new bytes container with the specified capacity.
    fn with_capacity(cap: usize) -> Self {
        Bytes {
            bytes: Vec::with_capacity(cap),
        }
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

    /// Optionally remove the last in the container.
    fn pop(&mut self) -> Option<u8> {
        self.bytes.pop()
    }
}

decl_external!(Bytes);

/// Install the bytes package.
pub fn install(functions: &mut Functions) -> Result<(), RegisterError> {
    let module = functions.module_mut(&["bytes"])?;
    module.global_fn("new", Bytes::new)?;
    module.global_fn("with_capacity", Bytes::with_capacity)?;

    let module = functions.global_module_mut();
    module.instance_fn("extend", Bytes::extend)?;
    module.instance_fn("extend_str", Bytes::extend_str)?;
    module.instance_fn("len", Bytes::len)?;
    module.instance_fn("clear", Bytes::clear)?;
    module.instance_fn("pop", Bytes::pop)?;
    module.instance_fn("clone", Bytes::clone)?;
    Ok(())
}
