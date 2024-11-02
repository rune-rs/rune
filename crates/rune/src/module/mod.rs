//! Types used for defining native modules.
//!
//! A native module is one that provides rune with functions and types through
//! native Rust-based code.

pub(crate) mod module;

pub(crate) mod install_with;
#[doc(inline)]
pub use self::install_with::InstallWith;

mod module_meta;
pub(crate) use self::module_meta::{
    AssociatedKey, DocFunction, Fields, ModuleAssociated, ModuleAssociatedKind, ModuleFunction,
    ModuleItem, ModuleItemCommon, ModuleItemKind, ModuleReexport, ModuleTrait, ModuleTraitImpl,
    ModuleType, TraitFunction, TypeSpecification,
};
use self::module_meta::{Enum, ModuleAttributeMacro, ModuleMacro, Variant};
#[doc(inline)]
pub use self::module_meta::{ModuleMeta, ModuleMetaData};

mod item_mut;
#[doc(inline)]
pub use self::item_mut::ItemMut;

mod trait_mut;
#[doc(inline)]
pub use self::trait_mut::TraitMut;

mod item_fn_mut;
#[doc(inline)]
pub use self::item_fn_mut::ItemFnMut;

mod variant_mut;
#[doc(inline)]
pub use self::variant_mut::VariantMut;

mod enum_mut;
#[doc(inline)]
pub use self::enum_mut::EnumMut;

mod type_mut;
#[doc(inline)]
pub use self::type_mut::TypeMut;

mod module_function_builder;
#[doc(inline)]
pub use self::module_function_builder::ModuleFunctionBuilder;

mod module_raw_function_builder;
#[doc(inline)]
pub use self::module_raw_function_builder::ModuleRawFunctionBuilder;

mod module_constant_builder;
#[doc(inline)]
pub use self::module_constant_builder::ModuleConstantBuilder;

#[doc(inline)]
pub use self::module::Module;
