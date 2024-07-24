//! Types used for defining native modules.
//!
//! A native module is one that provides rune with functions and types through
//! native Rust-based code.

pub(crate) mod module;

pub(crate) mod install_with;
pub use self::install_with::InstallWith;

mod internal_enum;
use self::internal_enum::InternalEnum;

mod module_meta;
pub(crate) use self::module_meta::{
    AssociatedKey, Fields, ModuleAssociated, ModuleAssociatedKind, ModuleItem, ModuleItemKind,
    ModuleType, TypeSpecification,
};
use self::module_meta::{
    Enum, ModuleAttributeMacro, ModuleFunction, ModuleItemCommon, ModuleMacro, Variant,
};

mod item_mut;
pub use self::item_mut::ItemMut;

mod item_fn_mut;
pub use self::item_fn_mut::ItemFnMut;

mod variant_mut;
pub use self::variant_mut::VariantMut;

mod enum_mut;
pub use self::enum_mut::EnumMut;

mod internal_enum_mut;
pub use self::internal_enum_mut::InternalEnumMut;

mod type_mut;
pub use self::type_mut::TypeMut;

#[doc(hidden)]
pub use self::module::{
    Module, ModuleConstantBuilder, ModuleFunctionBuilder, ModuleMeta, ModuleMetaData,
    ModuleRawFunctionBuilder,
};
