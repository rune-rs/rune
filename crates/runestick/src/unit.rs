//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::collections::HashMap;
use crate::{Call, DebugInfo, Hash, Inst, StaticString, Type, VmError, VmErrorKind};
use std::fmt;
use std::sync::Arc;

/// Instructions from a single source file.
#[derive(Debug, Default)]
pub struct Unit {
    /// The instructions contained in the source file.
    pub(crate) instructions: Vec<Inst>,
    /// Where functions are located in the collection of instructions.
    pub(crate) functions: HashMap<Hash, UnitFn>,
    /// Declared types.
    pub(crate) types: HashMap<Hash, UnitTypeInfo>,
    /// A static string.
    pub(crate) static_strings: Vec<Arc<StaticString>>,
    /// A static byte string.
    pub(crate) static_bytes: Vec<Vec<u8>>,
    /// Slots used for object keys.
    ///
    /// This is used when an object is used in a pattern match, to avoid having
    /// to send the collection of keys to the virtual machine.
    ///
    /// All keys are sorted with the default string sort.
    pub(crate) static_object_keys: Vec<Box<[String]>>,
    /// Debug info if available for unit.
    pub(crate) debug: Option<Box<DebugInfo>>,
}

impl Unit {
    /// Construct a new unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Access the type for the given language item.
    pub fn lookup_type(&self, hash: Hash) -> Option<&UnitTypeInfo> {
        self.types.get(&hash)
    }

    /// Access debug information for the given location if it is available.
    pub fn debug_info(&self) -> Option<&DebugInfo> {
        let debug = self.debug.as_ref()?;
        Some(&**debug)
    }

    /// Get the instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<&Inst> {
        self.instructions.get(ip)
    }

    /// Iterate over all static strings in the unit.
    pub fn iter_static_strings(&self) -> impl Iterator<Item = &Arc<StaticString>> + '_ {
        self.static_strings.iter()
    }

    /// Iterate over all static object keys in the unit.
    pub fn iter_static_object_keys(&self) -> impl Iterator<Item = (Hash, &[String])> + '_ {
        let mut it = self.static_object_keys.iter();

        std::iter::from_fn(move || {
            let s = it.next()?;
            Some((Hash::object_keys(&s[..]), &s[..]))
        })
    }

    /// Iterate over all instructions in order.
    pub fn iter_instructions(&self) -> impl Iterator<Item = Inst> + '_ {
        self.instructions.iter().copied()
    }

    /// Iterate over known functions.
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &UnitFn)> + '_ {
        let mut it = self.functions.iter();

        std::iter::from_fn(move || {
            let (k, v) = it.next()?;
            Some((*k, v))
        })
    }

    /// Lookup the static string by slot, if it exists.
    pub fn lookup_string(&self, slot: usize) -> Result<&Arc<StaticString>, VmError> {
        Ok(self
            .static_strings
            .get(slot)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticString { slot }))?)
    }

    /// Lookup the static byte string by slot, if it exists.
    pub fn lookup_bytes(&self, slot: usize) -> Result<&[u8], VmError> {
        Ok(self
            .static_bytes
            .get(slot)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticString { slot }))?
            .as_ref())
    }

    /// Lookup the static object keys by slot, if it exists.
    pub fn lookup_object_keys(&self, slot: usize) -> Option<&[String]> {
        self.static_object_keys.get(slot).map(|keys| &keys[..])
    }

    /// Lookup information of a function.
    pub fn lookup(&self, hash: Hash) -> Option<UnitFn> {
        self.functions.get(&hash).copied()
    }
}

/// The kind and necessary information on registered functions.
#[derive(Debug, Clone, Copy)]
pub enum UnitFn {
    /// Offset to call a "real" function.
    Offset {
        /// Offset of the registered function.
        offset: usize,
        /// The way the function is called.
        call: Call,
        /// The number of arguments the function takes.
        args: usize,
    },
    /// A tuple constructor.
    Tuple {
        /// The type of the tuple.
        hash: Hash,
        /// The number of arguments the tuple takes.
        args: usize,
    },
    /// A tuple variant constructor.
    TupleVariant {
        /// The hash of the enum type.
        enum_hash: Hash,
        /// The hash of the variant.
        hash: Hash,
        /// The number of arguments the tuple takes.
        args: usize,
    },
}

impl fmt::Display for UnitFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset { offset, call, args } => {
                write!(f, "offset {}, {}, {}", offset, call, args)?;
            }
            Self::Tuple { hash, args } => {
                write!(f, "tuple {}, {}", hash, args)?;
            }
            Self::TupleVariant {
                enum_hash,
                hash,
                args,
            } => {
                write!(f, "tuple-variant {}, {}, {}", enum_hash, hash, args)?;
            }
        }

        Ok(())
    }
}

/// Information on a type.
#[derive(Debug)]
pub struct UnitTypeInfo {
    /// A type declared in a unit.
    pub hash: Hash,
    /// value type of the given type.
    pub value_type: Type,
}
