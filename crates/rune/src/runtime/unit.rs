//! A single execution unit in the rune virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

#[cfg(feature = "byte-code")]
mod byte_code;
mod storage;

use core::fmt;

use ::rust_alloc::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, String, Vec};
use crate::hash;
use crate::runtime::{Call, ConstValue, DebugInfo, Inst, InstAddress, Rtti, StaticString};
use crate::Hash;

pub use self::storage::{ArrayUnit, EncodeError, UnitEncoder, UnitStorage};
pub(crate) use self::storage::{BadInstruction, BadJump};

#[cfg(feature = "byte-code")]
pub use self::byte_code::ByteCodeUnit;

/// Default storage implementation to use.
#[cfg(not(rune_byte_code))]
pub type DefaultStorage = ArrayUnit;
/// Default storage implementation to use.
#[cfg(rune_byte_code)]
pub type DefaultStorage = ByteCodeUnit;

/// Instructions and debug info from a single compilation.
///
/// See [`rune::prepare`] for more.
#[derive(Debug, TryClone, Default, Serialize, Deserialize)]
#[serde(bound = "S: Serialize + DeserializeOwned")]
#[try_clone(bound = {S: TryClone})]
pub struct Unit<S = DefaultStorage> {
    /// The information needed to execute the program.
    #[serde(flatten)]
    logic: Logic<S>,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
}

/// Instructions from a single source file.
#[derive(Debug, TryClone, Default, Serialize, Deserialize)]
#[serde(rename = "Unit")]
#[try_clone(bound = {S: TryClone})]
pub struct Logic<S = DefaultStorage> {
    /// Storage for the unit.
    storage: S,
    /// Where functions are located in the collection of instructions.
    functions: hash::Map<UnitFn>,
    /// Static strings.
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
    /// Drop sets.
    drop_sets: Vec<Arc<[InstAddress]>>,
    /// Runtime information for types.
    rtti: hash::Map<Arc<Rtti>>,
    /// Named constants
    constants: hash::Map<ConstValue>,
}

impl<S> Unit<S> {
    /// Constructs a new unit from a pair of data and debug info.
    #[inline]
    pub fn from_parts(data: Logic<S>, debug: Option<DebugInfo>) -> alloc::Result<Self> {
        Ok(Self {
            logic: data,
            debug: debug.map(Box::try_new).transpose()?,
        })
    }

    /// Construct a new unit with the given content.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub(crate) fn new(
        storage: S,
        functions: hash::Map<UnitFn>,
        static_strings: Vec<Arc<StaticString>>,
        static_bytes: Vec<Vec<u8>>,
        static_object_keys: Vec<Box<[String]>>,
        drop_sets: Vec<Arc<[InstAddress]>>,
        rtti: hash::Map<Arc<Rtti>>,
        debug: Option<Box<DebugInfo>>,
        constants: hash::Map<ConstValue>,
    ) -> Self {
        Self {
            logic: Logic {
                storage,
                functions,
                static_strings,
                static_bytes,
                static_object_keys,
                drop_sets,
                rtti,
                constants,
            },
            debug,
        }
    }

    /// Access unit data.
    #[inline]
    pub fn logic(&self) -> &Logic<S> {
        &self.logic
    }

    /// Access debug information for the given location if it is available.
    #[inline]
    pub fn debug_info(&self) -> Option<&DebugInfo> {
        Some(&**self.debug.as_ref()?)
    }

    /// Get raw underlying instructions storage.
    #[inline]
    pub(crate) fn instructions(&self) -> &S {
        &self.logic.storage
    }

    /// Iterate over all static strings in the unit.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_static_strings(&self) -> impl Iterator<Item = &Arc<StaticString>> + '_ {
        self.logic.static_strings.iter()
    }

    /// Iterate over all static bytes in the unit.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_static_bytes(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.logic.static_bytes.iter().map(|v| &**v)
    }

    /// Iterate over all available drop sets.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_static_drop_sets(&self) -> impl Iterator<Item = &[InstAddress]> + '_ {
        self.logic.drop_sets.iter().map(|v| &**v)
    }

    /// Iterate over all constants in the unit.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_constants(&self) -> impl Iterator<Item = (&Hash, &ConstValue)> + '_ {
        self.logic.constants.iter()
    }

    /// Iterate over all static object keys in the unit.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_static_object_keys(&self) -> impl Iterator<Item = (usize, &[String])> + '_ {
        use core::iter;

        let mut it = self.logic.static_object_keys.iter().enumerate();

        iter::from_fn(move || {
            let (n, s) = it.next()?;
            Some((n, &s[..]))
        })
    }

    /// Iterate over dynamic functions.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter_functions(&self) -> impl Iterator<Item = (Hash, &UnitFn)> + '_ {
        self.logic.functions.iter().map(|(h, f)| (*h, f))
    }

    /// Lookup the static string by slot, if it exists.
    #[inline]
    pub(crate) fn lookup_string(&self, slot: usize) -> Option<&Arc<StaticString>> {
        self.logic.static_strings.get(slot)
    }

    /// Lookup the static byte string by slot, if it exists.
    #[inline]
    pub(crate) fn lookup_bytes(&self, slot: usize) -> Option<&[u8]> {
        Some(self.logic.static_bytes.get(slot)?)
    }

    /// Lookup the static object keys by slot, if it exists.
    #[inline]
    pub(crate) fn lookup_object_keys(&self, slot: usize) -> Option<&[String]> {
        Some(self.logic.static_object_keys.get(slot)?)
    }

    #[inline]
    pub(crate) fn lookup_drop_set(&self, set: usize) -> Option<&[InstAddress]> {
        Some(self.logic.drop_sets.get(set)?)
    }

    /// Lookup run-time information for the given type hash.
    #[inline]
    pub(crate) fn lookup_rtti(&self, hash: &Hash) -> Option<&Arc<Rtti>> {
        self.logic.rtti.get(hash)
    }

    /// Lookup a function in the unit.
    #[inline]
    pub(crate) fn function(&self, hash: &Hash) -> Option<&UnitFn> {
        self.logic.functions.get(hash)
    }

    /// Lookup a constant from the unit.
    #[inline]
    pub(crate) fn constant(&self, hash: &Hash) -> Option<&ConstValue> {
        self.logic.constants.get(hash)
    }
}

impl<S> Unit<S>
where
    S: UnitStorage,
{
    #[inline]
    pub(crate) fn translate(&self, jump: usize) -> Result<usize, BadJump> {
        self.logic.storage.translate(jump)
    }

    /// Get the instruction at the given instruction pointer.
    #[inline]
    pub(crate) fn instruction_at(
        &self,
        ip: usize,
    ) -> Result<Option<(Inst, usize)>, BadInstruction> {
        self.logic.storage.get(ip)
    }

    /// Iterate over all instructions in order.
    #[cfg(feature = "emit")]
    #[inline]
    pub(crate) fn iter_instructions(&self) -> impl Iterator<Item = (usize, Inst)> + '_ {
        self.logic.storage.iter()
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
        /// If the offset is a closure, this indicates the number of captures in
        /// the first argument.
        captures: Option<usize>,
    },
    /// An empty constructor of the type identified by the given hash.
    EmptyStruct {
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
}

impl TryClone for UnitFn {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(*self)
    }
}

impl fmt::Display for UnitFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset {
                offset,
                call,
                args,
                captures,
            } => {
                write!(
                    f,
                    "offset offset={offset}, call={call}, args={args}, captures={captures:?}"
                )?;
            }
            Self::EmptyStruct { hash } => {
                write!(f, "unit hash={hash}")?;
            }
            Self::TupleStruct { hash, args } => {
                write!(f, "tuple hash={hash}, args={args}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(Unit: Send, Sync);
