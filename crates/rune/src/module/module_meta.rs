use core::fmt;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::compile::context::{AttributeMacroHandler, MacroHandler, TraitHandler};
use crate::compile::{meta, Docs};
use crate::function_meta::AssociatedName;
use crate::runtime::{ConstValue, FunctionHandler, TypeCheck, TypeInfo};
use crate::{Hash, ItemBuf};

use super::InternalEnum;

#[doc(hidden)]
pub struct ModuleMetaData {
    #[doc(hidden)]
    pub item: ItemBuf,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
}

/// Type used to collect and store module metadata through the `#[rune::module]`
/// macro.
///
/// This is the argument type for [`Module::from_meta`], and is from a public
/// API perspective completely opaque and might change for any release.
///
/// Calling and making use of `ModuleMeta` manually despite this warning might
/// lead to future breakage.
///
/// [`Module::from_meta`]: crate::Module::from_meta
pub type ModuleMeta = fn() -> alloc::Result<ModuleMetaData>;

/// Data for an opaque type. If `spec` is set, indicates things which are known
/// about that type.
pub(crate) struct ModuleType {
    /// The name of the installed type which will be the final component in the
    /// item it will constitute.
    pub(crate) item: ItemBuf,
    /// Type hash of the type.
    pub(crate) hash: Hash,
    /// Common item metadata.
    pub(crate) common: ModuleItemCommon,
    /// Type parameters for this item.
    pub(crate) type_parameters: Hash,
    /// Type information for the installed type.
    pub(crate) type_info: TypeInfo,
    /// The specification for the type.
    pub(crate) spec: Option<TypeSpecification>,
    /// Handler to use if this type can be constructed through a regular function call.
    pub(crate) constructor: Option<Arc<FunctionHandler>>,
}

/// A trait defined in a module.
pub(crate) struct ModuleTrait {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) common: ModuleItemCommon,
    pub(crate) handler: Option<Arc<TraitHandler>>,
    pub(crate) functions: Vec<TraitFunction>,
}

/// A type implementing a trait.
pub(crate) struct ModuleTraitImpl {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) type_info: TypeInfo,
    pub(crate) trait_item: ItemBuf,
    pub(crate) trait_hash: Hash,
}

/// A reexport of an item.
pub(crate) struct ModuleReexport {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) to: ItemBuf,
}

/// The kind of the variant.
#[derive(Debug)]
pub(crate) enum Fields {
    /// Sequence of named fields.
    Named(&'static [&'static str]),
    /// Sequence of unnamed fields.
    Unnamed(usize),
    /// Empty.
    Empty,
}

/// Metadata about a variant.
pub struct Variant {
    /// The name of the variant.
    pub(crate) name: &'static str,
    /// Type check for the variant.
    pub(crate) type_check: Option<TypeCheck>,
    /// Variant metadata.
    pub(crate) fields: Option<Fields>,
    /// Handler to use if this variant can be constructed through a regular function call.
    pub(crate) constructor: Option<Arc<FunctionHandler>>,
    /// Variant deprecation.
    pub(crate) deprecated: Option<Box<str>>,
    /// Variant documentation.
    pub(crate) docs: Docs,
}

impl Variant {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            type_check: None,
            fields: None,
            constructor: None,
            deprecated: None,
            docs: Docs::EMPTY,
        }
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Variant");
        f.field("fields", &self.fields);
        f.field("constructor", &self.constructor.is_some());
        #[cfg(feature = "doc")]
        f.field("deprecated", &self.deprecated);
        f.field("docs", &self.docs);
        f.finish()
    }
}

/// The type specification for a native enum.
pub(crate) struct Enum {
    /// The variants.
    pub(crate) variants: Vec<Variant>,
}

/// A type specification.
pub(crate) enum TypeSpecification {
    Struct(Fields),
    Enum(Enum),
}

/// A key that identifies an associated function.
#[derive(Debug, TryClone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) struct AssociatedKey {
    /// The type the associated function belongs to.
    pub(crate) type_hash: Hash,
    /// The kind of the associated function.
    pub(crate) kind: meta::AssociatedKind,
    /// The type parameters of the associated function.
    pub(crate) parameters: Hash,
}

pub(crate) enum ModuleItemKind {
    Constant(ConstValue),
    Function(ModuleFunction),
    Macro(ModuleMacro),
    AttributeMacro(ModuleAttributeMacro),
    InternalEnum(InternalEnum),
}

pub(crate) struct ModuleItem {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) common: ModuleItemCommon,
    pub(crate) kind: ModuleItemKind,
}

#[derive(Default, TryClone)]
pub(crate) struct DocFunction {
    #[cfg(feature = "doc")]
    #[try_clone(copy)]
    pub(crate) is_async: bool,
    #[cfg(feature = "doc")]
    #[try_clone(copy)]
    pub(crate) args: Option<usize>,
    #[cfg(feature = "doc")]
    pub(crate) argument_types: Box<[meta::DocType]>,
    #[cfg(feature = "doc")]
    pub(crate) return_type: meta::DocType,
}

#[derive(TryClone)]
pub(crate) struct ModuleFunction {
    /// The handler for the function.
    pub(crate) handler: Arc<FunctionHandler>,
    /// If the function is associated with a trait, this is the hash of that trait.
    pub(crate) trait_hash: Option<Hash>,
    /// Documentation related to the function.
    pub(crate) doc: DocFunction,
}

#[derive(TryClone)]
pub(crate) enum ModuleAssociatedKind {
    Constant(ConstValue),
    Function(ModuleFunction),
}

#[derive(Default, TryClone)]
pub(crate) struct ModuleItemCommon {
    /// Documentation for the item.
    pub(crate) docs: Docs,
    /// Deprecation marker for the item.
    pub(crate) deprecated: Option<Box<str>>,
}

#[derive(TryClone)]
pub(crate) struct ModuleAssociated {
    pub(crate) container: Hash,
    pub(crate) container_type_info: TypeInfo,
    pub(crate) name: AssociatedName,
    pub(crate) common: ModuleItemCommon,
    pub(crate) kind: ModuleAssociatedKind,
}

/// Handle to a macro inserted into a module.
pub(crate) struct ModuleMacro {
    pub(crate) handler: Arc<MacroHandler>,
}

/// Handle to an attribute macro inserted into a module.
pub(crate) struct ModuleAttributeMacro {
    pub(crate) handler: Arc<AttributeMacroHandler>,
}

/// Handle to a trait function inserted into a module.
pub(crate) struct TraitFunction {
    pub(crate) name: AssociatedName,
    pub(crate) common: ModuleItemCommon,
    pub(crate) doc: DocFunction,
}
