//! A single execution unit in the rune virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

#[cfg(feature = "byte-code")]
mod byte_code;
mod storage;

use core::fmt;

#[cfg(feature = "musli")]
use musli::mode::Binary;
#[cfg(feature = "musli")]
use musli::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::hash;
use crate::runtime::debug::{DebugArgs, DebugSignature};
use crate::runtime::{Address, Call, ConstValue, DebugInfo, Inst, Rtti, RttiKind, StaticString};
use crate::sync::Arc;
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
#[derive(Debug, TryClone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound = "S: Serialize + DeserializeOwned"))]
#[cfg_attr(feature = "musli", derive(Encode, Decode))]
#[cfg_attr(feature = "musli", musli(Binary, bound = {S: Encode<Binary>}, decode_bound<'de, A> = {S: Decode<'de, Binary, A>}))]
#[try_clone(bound = {S: TryClone})]
pub struct Unit<S = DefaultStorage> {
    /// The information needed to execute the program.
    #[cfg_attr(feature = "serde", serde(flatten))]
    logic: Logic<S>,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
}

assert_impl!(Unit<DefaultStorage>: Send + Sync);

/// Instructions from a single source file.
#[derive(Debug, TryClone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename = "Unit")
)]
#[cfg_attr(feature = "musli", derive(Encode, Decode))]
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
    drop_sets: Vec<Arc<[Address]>>,
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
        drop_sets: Vec<Arc<[Address]>>,
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
    pub(crate) fn iter_static_drop_sets(&self) -> impl Iterator<Item = &[Address]> + '_ {
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
    pub(crate) fn lookup_drop_set(&self, set: usize) -> Option<&[Address]> {
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

    /// Iterate over all function signatures with type information.
    ///
    /// Returns an iterator of [`FunctionSignature`] for all functions in the unit.
    ///
    /// [`FunctionSignature`]: crate::compile::type_info::FunctionSignature
    pub fn function_signatures(
        &self,
    ) -> impl Iterator<Item = crate::compile::type_info::FunctionSignature> + '_ {
        let debug = self.debug_info();
        debug
            .into_iter()
            .flat_map(|d| d.functions.iter())
            .filter_map(|(hash, sig)| self.build_function_signature(*hash, sig))
    }

    /// Get signature for a specific function by hash.
    ///
    /// Returns `None` if the function is not found or debug info is unavailable.
    pub fn function_signature(
        &self,
        hash: Hash,
    ) -> Option<crate::compile::type_info::FunctionSignature> {
        let debug = self.debug_info()?;
        let sig = debug.functions.get(&hash)?;
        self.build_function_signature(hash, sig)
    }

    /// Get signature for a function by name (last path component).
    ///
    /// Returns the first function found with the given name.
    pub fn function_signature_by_name(
        &self,
        name: &str,
    ) -> Option<crate::compile::type_info::FunctionSignature> {
        let debug = self.debug_info()?;
        for (hash, sig) in &debug.functions {
            let last_component = sig.path.last().and_then(|c| c.as_str());
            if last_component == Some(name) {
                return self.build_function_signature(*hash, sig);
            }
        }
        None
    }

    /// Build a FunctionSignature from a DebugSignature.
    fn build_function_signature(
        &self,
        hash: Hash,
        sig: &DebugSignature,
    ) -> Option<crate::compile::type_info::FunctionSignature> {
        use crate::alloc::prelude::*;
        use crate::compile::type_info::{FunctionSignature, ParameterType};

        let name = sig
            .path
            .last()
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .try_to_string()
            .ok()?;
        let path = sig.path.try_to_string().ok()?;

        // Build parameters from param_types if available, otherwise from args
        let parameters = if let Some(param_types) = &sig.param_types {
            let mut params = Vec::new();
            for (position, (param_name, type_str)) in param_types.iter().enumerate() {
                let type_info = type_str.as_ref().map(|s| parse_type_string(s));
                params
                    .try_push(ParameterType {
                        name: param_name.as_ref().try_to_string().ok()?,
                        type_info,
                        position,
                    })
                    .ok()?;
            }
            params
        } else {
            // Fall back to args for parameter names without types
            match &sig.args {
                DebugArgs::Named(names) => {
                    let mut params = Vec::new();
                    for (position, name) in names.iter().enumerate() {
                        params
                            .try_push(ParameterType {
                                name: name.as_ref().try_to_string().ok()?,
                                type_info: None,
                                position,
                            })
                            .ok()?;
                    }
                    params
                }
                DebugArgs::TupleArgs(count) => {
                    let mut params = Vec::new();
                    for position in 0..*count {
                        let mut name = String::new();
                        use crate::alloc::fmt::TryWrite;
                        write!(name, "arg{}", position).ok()?;
                        params
                            .try_push(ParameterType {
                                name,
                                type_info: None,
                                position,
                            })
                            .ok()?;
                    }
                    params
                }
                DebugArgs::EmptyArgs => Vec::new(),
            }
        };

        let return_type = sig.return_type.as_ref().map(|s| parse_type_string(s));

        Some(FunctionSignature {
            name,
            path,
            hash,
            is_async: sig.is_async,
            parameters,
            return_type,
        })
    }

    /// Iterate over all struct types with type information.
    ///
    /// Returns an iterator of [`StructInfo`] for all structs in the unit.
    ///
    /// [`StructInfo`]: crate::compile::type_info::StructInfo
    pub fn struct_infos(
        &self,
    ) -> impl Iterator<Item = crate::compile::type_info::StructInfo> + '_ {
        self.logic.rtti.iter().filter_map(|(hash, rtti)| {
            if matches!(rtti.kind, RttiKind::Struct) && rtti.variant_hash == Hash::EMPTY {
                self.build_struct_info(*hash, rtti)
            } else {
                None
            }
        })
    }

    /// Get type information for a struct by its hash.
    ///
    /// Returns `None` if the struct is not found or RTTI is unavailable.
    pub fn struct_info(&self, hash: Hash) -> Option<crate::compile::type_info::StructInfo> {
        let rtti = self.logic.rtti.get(&hash)?;
        if !matches!(rtti.kind, RttiKind::Struct) || rtti.variant_hash != Hash::EMPTY {
            return None;
        }
        self.build_struct_info(hash, rtti)
    }

    /// Get struct info by name (last path component).
    ///
    /// Returns the first struct found with the given name.
    pub fn struct_info_by_name(
        &self,
        name: &str,
    ) -> Option<crate::compile::type_info::StructInfo> {
        for (hash, rtti) in &self.logic.rtti {
            if !matches!(rtti.kind, RttiKind::Struct) || rtti.variant_hash != Hash::EMPTY {
                continue;
            }
            let last_component = rtti.item.last().and_then(|c| c.as_str());
            if last_component == Some(name) {
                return self.build_struct_info(*hash, rtti);
            }
        }
        None
    }

    /// Build a StructInfo from RTTI and debug information.
    fn build_struct_info(
        &self,
        hash: Hash,
        rtti: &Arc<Rtti>,
    ) -> Option<crate::compile::type_info::StructInfo> {
        use crate::alloc::prelude::*;
        use crate::compile::type_info::{FieldInfo, StructInfo};

        let name = Box::try_from(
            rtti.item
                .last()
                .and_then(|c| c.as_str())
                .unwrap_or("")
        ).ok()?;

        let path = rtti.item.try_clone().ok()?;

        // Get field type annotations from debug info if available
        let debug_struct = self.debug_info().and_then(|d| d.structs.get(&hash));

        let mut field_types_map: HashMap<&str, Option<&str>> = HashMap::new();
        if let Some(ds) = debug_struct {
            if let Some(field_types) = &ds.field_types {
                for (name, ty) in field_types.iter() {
                    let _ = field_types_map.try_insert(
                        name.as_ref(),
                        ty.as_ref().map(|t| t.as_ref())
                    );
                }
            }
        }

        // Build fields from RTTI with type annotations from debug info
        let mut fields = Vec::new();
        for (field_name, position) in &rtti.fields {
            let type_info = field_types_map
                .get(field_name.as_ref())
                .and_then(|ty_opt| *ty_opt)
                .map(parse_type_string);

            fields
                .try_push(FieldInfo {
                    name: field_name.try_clone().ok()?,
                    position: *position,
                    type_info,
                })
                .ok()?;
        }

        Some(StructInfo {
            name,
            path,
            hash,
            fields: Box::try_from(fields).ok()?,
        })
    }
}

/// Parse a type string into AnnotatedType.
fn parse_type_string(s: &str) -> crate::compile::type_info::AnnotatedType {
    use crate::compile::type_info::AnnotatedType;

    let s = s.trim();

    // Handle tuple types
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        if inner.is_empty() {
            return AnnotatedType::Tuple(Vec::new());
        }
        // Simple split by comma (doesn't handle nested tuples perfectly, but good enough for now)
        let mut parts = Vec::new();
        for p in inner.split(',') {
            let _ = parts.try_push(parse_type_string(p.trim()));
        }
        return AnnotatedType::Tuple(parts);
    }

    // Handle never type
    if s == "!" {
        return AnnotatedType::Never;
    }

    // Named type
    AnnotatedType::Named {
        path: s.try_into().unwrap_or_default(),
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
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Encode, Decode))]
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
