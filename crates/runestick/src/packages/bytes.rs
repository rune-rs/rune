//! The bytes package, providing access to the bytes type.

use crate::context::{ContextError, Module};
use crate::reflection::{ReflectValueType, UnsafeFromValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, Vm, VmError};
use std::any::{type_name, TypeId};
use std::fmt;
use std::ops;

/// A bytes container.
#[derive(Clone)]
pub struct Bytes {
    bytes: Vec<u8>,
}

impl ops::Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.bytes, fmt)
    }
}

impl Bytes {
    /// Convert into inner byte array.
    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

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

    /// Get the capacity of the bytes collection.
    fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Get the bytes collection.
    fn clear(&mut self) {
        self.bytes.clear();
    }

    fn reserve(&mut self, additional: usize) {
        self.bytes.reserve(additional);
    }

    fn reserve_exact(&mut self, additional: usize) {
        self.bytes.reserve_exact(additional);
    }

    fn shrink_to_fit(&mut self) {
        self.bytes.shrink_to_fit();
    }

    fn pop(&mut self) -> Option<u8> {
        self.bytes.pop()
    }

    fn last(&mut self) -> Option<u8> {
        self.bytes.last().copied()
    }
}

decl_external!(Bytes);

impl<'a> UnsafeFromValue for &'a [u8] {
    type Output = *const [u8];
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_external(vm)?;
        let (value, guard) = Ref::unsafe_into_ref(vm.external_ref::<Bytes>(slot)?);
        Ok(((*value).bytes.as_slice(), guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> ReflectValueType for &'a [u8] {
    type Owned = Bytes;

    fn value_type() -> ValueType {
        ValueType::External(TypeId::of::<Bytes>())
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::External(type_name::<Bytes>())
    }
}

/// Get the module for the bytes package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "bytes"]);

    module.ty(&["Bytes"]).build::<Bytes>()?;
    module.function(&["Bytes", "new"], Bytes::new)?;
    module.function(&["Bytes", "with_capacity"], Bytes::with_capacity)?;

    module.inst_fn("extend", Bytes::extend)?;
    module.inst_fn("extend_str", Bytes::extend_str)?;
    module.inst_fn("pop", Bytes::pop)?;
    module.inst_fn("last", Bytes::last)?;

    module.inst_fn("len", Bytes::len)?;
    module.inst_fn("capacity", Bytes::capacity)?;
    module.inst_fn("clear", Bytes::clear)?;
    module.inst_fn("reserve", Bytes::reserve)?;
    module.inst_fn("reserve_exact", Bytes::reserve_exact)?;
    module.inst_fn("clone", Bytes::clone)?;
    module.inst_fn("shrink_to_fit", Bytes::shrink_to_fit)?;
    Ok(module)
}
