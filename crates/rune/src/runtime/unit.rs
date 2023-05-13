//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use core::fmt;

use crate::no_std::prelude::*;

use serde::{Deserialize, Serialize};

use crate::collections::HashMap;
use crate::no_std::sync::Arc;
use crate::runtime::{
    Call, ConstValue, DebugInfo, Inst, Rtti, StaticString, VariantRtti, VmError, VmErrorKind,
};
use crate::Hash;

/// Instructions from a single source file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Unit {
    /// The instructions contained in the source file.
    instructions: Vec<u8>,
    /// Known jump offsets.
    offsets: Vec<usize>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<Hash, UnitFn>,
    /// A static string.
    static_strings: Vec<Arc<StaticString>>,
    /// A static byte string.
    static_bytes: Vec<Vec<u8>>,
    /// Slots used for object keys.
    ///
    /// This is used when an object is used in a pattern match, to avoid having
    /// to send the collection of keys to the virtual machine.
    ///
    /// All keys are sorted with the default string sort.
    static_object_keys: Vec<Box<[String]>>,
    /// Runtime information for types.
    rtti: HashMap<Hash, Arc<Rtti>>,
    /// Runtime information for variants.
    variant_rtti: HashMap<Hash, Arc<VariantRtti>>,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
    /// Named constants
    constants: HashMap<Hash, ConstValue>,
}

impl Unit {
    /// Construct a new unit with the given content.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        instructions: Vec<u8>,
        offsets: Vec<usize>,
        functions: HashMap<Hash, UnitFn>,
        static_strings: Vec<Arc<StaticString>>,
        static_bytes: Vec<Vec<u8>>,
        static_object_keys: Vec<Box<[String]>>,
        rtti: HashMap<Hash, Arc<Rtti>>,
        variant_rtti: HashMap<Hash, Arc<VariantRtti>>,
        debug: Option<Box<DebugInfo>>,
        constants: HashMap<Hash, ConstValue>,
    ) -> Self {
        Self {
            instructions,
            offsets,
            functions,
            static_strings,
            static_bytes,
            static_object_keys,
            rtti,
            variant_rtti,
            debug,
            constants,
        }
    }

    #[inline]
    pub(crate) fn offset(&self, jump: usize) -> Option<usize> {
        if let Some(&offset) = self.offsets.get(jump) {
            Some(offset)
        } else {
            None
        }
    }

    /// Access debug information for the given location if it is available.
    pub fn debug_info(&self) -> Option<&DebugInfo> {
        let debug = self.debug.as_ref()?;
        Some(&**debug)
    }

    /// Get raw underying instructions.
    pub(crate) fn instructions(&self) -> &[u8] {
        &self.instructions
    }

    /// Get the instruction at the given instruction pointer.
    pub(crate) fn instruction_at(&self, ip: usize) -> Result<Option<(Inst, usize)>, VmErrorKind> {
        let Some(mut bytes) = self.instructions.get(ip..) else {
            return Ok(None);
        };

        let start = bytes.as_ptr();
        let inst: Inst =
            musli_storage::decode(&mut bytes).map_err(|_| VmErrorKind::BadInstruction)?;
        let len = (bytes.as_ptr() as usize).wrapping_sub(start as usize);
        Ok(Some((inst, len)))
    }

    /// Iterate over all static strings in the unit.
    #[cfg(feature = "cli")]
    pub(crate) fn iter_static_strings(&self) -> impl Iterator<Item = &Arc<StaticString>> + '_ {
        self.static_strings.iter()
    }

    /// Iterate over all constants in the unit.
    #[cfg(feature = "cli")]
    pub(crate) fn iter_constants(&self) -> impl Iterator<Item = (&Hash, &ConstValue)> + '_ {
        self.constants.iter()
    }

    /// Iterate over all static object keys in the unit.
    #[cfg(feature = "cli")]
    pub(crate) fn iter_static_object_keys(&self) -> impl Iterator<Item = (usize, &[String])> + '_ {
        use core::iter;

        let mut it = self.static_object_keys.iter().enumerate();

        iter::from_fn(move || {
            let (n, s) = it.next()?;
            Some((n, &s[..]))
        })
    }

    /// Iterate over all instructions in order.
    #[cfg(feature = "emit")]
    pub(crate) fn iter_instructions(&self) -> impl Iterator<Item = (usize, Inst)> + '_ {
        let mut address = &self.instructions[..];
        let len = address.len();

        std::iter::from_fn(move || {
            if address.is_empty() {
                return None;
            }

            let o = len - address.len();
            let inst = musli_storage::decode(&mut address).ok()?;
            Some((o, inst))
        })
    }

    /// Iterate over dynamic functions.
    #[cfg(feature = "cli")]
    pub(crate) fn iter_functions(&self) -> impl Iterator<Item = (Hash, &UnitFn)> + '_ {
        self.functions.iter().map(|(h, f)| (*h, f))
    }

    /// Lookup the static string by slot, if it exists.
    pub(crate) fn lookup_string(&self, slot: usize) -> Result<&Arc<StaticString>, VmError> {
        Ok(self
            .static_strings
            .get(slot)
            .ok_or(VmErrorKind::MissingStaticString { slot })?)
    }

    /// Lookup the static byte string by slot, if it exists.
    pub(crate) fn lookup_bytes(&self, slot: usize) -> Result<&[u8], VmError> {
        Ok(self
            .static_bytes
            .get(slot)
            .ok_or(VmErrorKind::MissingStaticString { slot })?
            .as_ref())
    }

    /// Lookup the static object keys by slot, if it exists.
    pub(crate) fn lookup_object_keys(&self, slot: usize) -> Option<&[String]> {
        self.static_object_keys.get(slot).map(|keys| &keys[..])
    }

    /// Lookup runt-time information for the given type hash.
    pub(crate) fn lookup_rtti(&self, hash: Hash) -> Option<&Arc<Rtti>> {
        self.rtti.get(&hash)
    }

    /// Lookup variant runt-time information for the given variant hash.
    pub(crate) fn lookup_variant_rtti(&self, hash: Hash) -> Option<&Arc<VariantRtti>> {
        self.variant_rtti.get(&hash)
    }

    /// Lookup a function in the unit.
    pub(crate) fn function(&self, hash: Hash) -> Option<UnitFn> {
        self.functions.get(&hash).copied()
    }

    /// Lookup a constant from the unit.
    pub(crate) fn constant(&self, hash: Hash) -> Option<&ConstValue> {
        self.constants.get(&hash)
    }
}

/// The kind and necessary information on registered functions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum UnitFn {
    /// Instruction offset of a function inside of the unit.
    Offset {
        /// Offset of the registered function.
        offset: usize,
        /// The way the function is called.
        call: Call,
        /// The number of arguments the function takes.
        args: usize,
    },
    /// An empty constructor of the type identified by the given hash.
    UnitStruct {
        /// The type hash of the empty.
        hash: Hash,
    },
    /// A tuple constructor of the type identified by the given hash.
    TupleStruct {
        /// The type hash of the tuple.
        hash: Hash,
        /// The number of arguments the tuple takes.
        args: usize,
    },
    /// A unit variant of the type identified by the given hash.
    UnitVariant {
        /// The type hash of the empty variant.
        hash: Hash,
    },
    /// A tuple variant of the type identified by the given hash.
    TupleVariant {
        /// The type hash of the variant.
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
            Self::UnitStruct { hash } => {
                write!(f, "unit {}", hash)?;
            }
            Self::TupleStruct { hash, args } => {
                write!(f, "tuple {}, {}", hash, args)?;
            }
            Self::UnitVariant { hash } => {
                write!(f, "empty-variant {}", hash)?;
            }
            Self::TupleVariant { hash, args } => {
                write!(f, "tuple-variant {}, {}", hash, args)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(Unit: Send, Sync);
