use crate::collections::{HashMap, HashSet};
use crate::module::{ModuleAssociatedFn, ModuleFn, ModuleInternalEnum, ModuleType, ModuleUnitType};
use crate::{
    Component, Hash, Item, Meta, MetaStruct, MetaTuple, Module, Names, Stack, StaticType, Type,
    TypeCheck, TypeInfo, UnitBuilder, ValueType, VmError,
};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

/// An error raised when building the context.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Conflicting `()` types.
    #[error("`()` types are already present")]
    UnitAlreadyPresent,
    /// Conflicting internal type.
    #[error("`{name}` types are already present")]
    InternalAlreadyPresent {
        /// The name of the internal type already present.
        name: &'static str,
    },
    /// A conflicting name.
    #[error("conflicting item `{item}`, inserted `{current}` while `{existing}` already existed")]
    ConflictingMeta {
        /// The item that conflicted
        item: Item,
        /// The current meta we tried to insert.
        current: Box<Meta>,
        /// The existing meta item.
        existing: Box<Meta>,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        /// The signature of the conflicting function.
        signature: ContextSignature,
        /// The hash of the conflicting function.
        hash: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function with name `{name}` already exists")]
    ConflictingFunctionName {
        /// The name of the conflicting function.
        name: Item,
    },
    /// Error raised when attempting to register a conflicting instance function.
    #[error("instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction {
        /// Type that we register the instance function for.
        type_info: TypeInfo,
        /// The name of the conflicting function.
        name: String,
    },
    /// Tried to insert a module that conflicted with an already existing one.
    #[error("module `{name}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        name: Item,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
    /// Raised when we try to register a conflicting type.
    #[error("type with name `{name}` already exists `{existing}`")]
    ConflictingType {
        /// The name we tried to register.
        name: Item,
        /// The type information for the type that already existed.
        existing: TypeInfo,
    },
    /// Raised when we try to register a conflicting type hash.
    #[error("tried to insert conflicting hash type `{hash}` (existing `{existing}`) for type `{value_type}`")]
    ConflictingTypeHash {
        /// The hash we are trying to insert.
        hash: Hash,
        /// The hash that already existed.
        existing: Hash,
        /// The type we're trying to insert.
        value_type: Type,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("variant with name `{name}` already exists")]
    ConflictingVariant {
        /// The name of the conflicting variant.
        name: Item,
    },
    /// Error raised when attempting to register an instance function on an
    /// instance which does not exist.
    #[error("instance `{instance_type}` does not exist in module")]
    MissingInstance {
        /// The instance type.
        instance_type: TypeInfo,
    },
    /// Error raised when attempting to register a type that doesn't have a type
    /// hash into a context.
    #[error("type `{value_type}` cannot be defined dynamically")]
    UnsupportedValueType {
        /// The type we tried to register.
        value_type: Type,
    },
}

/// A function handler.
pub(crate) type Handler = dyn Fn(&mut Stack, usize) -> Result<(), VmError> + Sync;

/// Information on a specific type.
#[derive(Debug, Clone)]
pub struct ContextTypeInfo {
    /// The type check used for the current type.
    ///
    /// If absent, the type cannot be type checked for.
    pub type_check: TypeCheck,
    /// The name of the type.
    pub name: Item,
    /// The value type of the type.
    pub value_type: Type,
    /// Information on the type.
    pub type_info: TypeInfo,
}

impl fmt::Display for ContextTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.name, self.type_info)?;
        Ok(())
    }
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub enum ContextSignature {
    Function {
        /// Path to the function.
        path: Item,
        /// Arguments.
        args: Option<usize>,
    },
    Instance {
        /// Path to the instance function.
        path: Item,
        /// Name of the instance function.
        name: String,
        /// Arguments.
        args: Option<usize>,
        /// Information on the self type.
        self_type_info: TypeInfo,
    },
}

impl fmt::Display for ContextSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function { path, args } => {
                write!(fmt, "{}(", path)?;

                if let Some(args) = args {
                    let mut it = 0..*args;
                    let last = it.next_back();

                    for n in it {
                        write!(fmt, "#{}, ", n)?;
                    }

                    if let Some(n) = last {
                        write!(fmt, "#{}", n)?;
                    }
                } else {
                    write!(fmt, "...")?;
                }

                write!(fmt, ")")?;
            }
            Self::Instance {
                path,
                name,
                self_type_info,
                args,
            } => {
                write!(fmt, "{}::{}(self: {}", path, name, self_type_info)?;

                if let Some(args) = args {
                    for n in 0..*args {
                        write!(fmt, ", #{}", n)?;
                    }
                } else {
                    write!(fmt, ", ...")?;
                }

                write!(fmt, ")")?;
            }
        }

        Ok(())
    }
}

/// Static run context visible to the virtual machine.
///
/// This contains:
/// * Declared functions.
/// * Declared instance functions.
/// * Type definitions.
#[derive(Default)]
pub struct Context {
    /// Whether or not to include the prelude when constructing a new unit.
    with_prelude: bool,
    /// Item metadata in the context.
    meta: HashMap<Item, Meta>,
    /// Registered native function handlers.
    functions: HashMap<Hash, Arc<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, ContextSignature>,
    /// Registered types.
    types: HashMap<Hash, ContextTypeInfo>,
    /// Reverse lookup for types.
    types_rev: HashMap<Type, Hash>,
    /// Specialized information on unit types, if available.
    unit_type: Option<Hash>,
    /// Registered internal enums.
    internal_enums: HashSet<&'static StaticType>,
    /// All available names in the context.
    names: Names,
}

impl Context {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Context::default()
    }

    /// Use the specified type check.
    pub fn type_check_for(&self, item: &Item) -> Option<TypeCheck> {
        let ty = self.types.get(&Hash::type_hash(item))?;
        Some(ty.type_check)
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        let mut this = Self::new();
        this.install(&crate::modules::core::module()?)?;
        this.install(&crate::modules::generator::module()?)?;
        this.install(&crate::modules::bytes::module()?)?;
        this.install(&crate::modules::string::module()?)?;
        this.install(&crate::modules::int::module()?)?;
        this.install(&crate::modules::float::module()?)?;
        this.install(&crate::modules::test::module()?)?;
        this.install(&crate::modules::iter::module()?)?;
        this.install(&crate::modules::vec::module()?)?;
        this.install(&crate::modules::object::module()?)?;
        this.install(&crate::modules::result::module()?)?;
        this.install(&crate::modules::option::module()?)?;
        this.install(&crate::modules::future::module()?)?;
        this.install(&crate::modules::stream::module()?)?;
        this.install(&crate::modules::io::module()?)?;
        this.install(&crate::modules::fmt::module()?)?;
        this.with_prelude = true;
        Ok(this)
    }

    /// Construct a new unit based on this context.
    ///
    /// What this does is primarily determined by how the context was
    /// constructed. If it was constructed through [with_default_modules], then
    /// this will construct a unit with a default prelude.
    ///
    /// Otherwise an empty unit will be constructed.
    ///
    /// [with_default_modules]: Self::with_default_modules
    pub fn new_unit(&self) -> UnitBuilder {
        if self.with_prelude {
            UnitBuilder::with_default_prelude()
        } else {
            UnitBuilder::default()
        }
    }

    /// Iterate over known child components of the given name.
    pub fn iter_components<'a, I>(&'a self, iter: I) -> impl Iterator<Item = &'a Component>
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        self.names.iter_components(iter)
    }

    /// Access the currently known unit type.
    pub fn unit_type(&self) -> Option<Hash> {
        self.unit_type
    }

    /// Check if unit contains the given name.
    pub fn contains_name(&self, item: &Item) -> bool {
        self.names.contains(item)
    }

    /// Check if unit contains the given name by prefix.
    pub fn contains_prefix(&self, item: &Item) -> bool {
        self.names.contains_prefix(item)
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(&self, name: &Item) -> Option<Meta> {
        self.meta.get(name).cloned()
    }

    /// Iterate over all available functions
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &ContextSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Iterate over all available types.
    pub fn iter_types(&self) -> impl Iterator<Item = (Hash, &ContextTypeInfo)> {
        let mut it = self.types.iter();

        std::iter::from_fn(move || {
            let (hash, ty) = it.next()?;
            Some((*hash, ty))
        })
    }

    /// Install the specified module.
    pub fn install(&mut self, module: &Module) -> Result<(), ContextError> {
        for (value_type, ty) in &module.types {
            self.install_type(&module, *value_type, ty)?;
        }

        for (name, f) in &module.functions {
            self.install_function(&module, name, f)?;
        }

        if let Some(unit_type) = &module.unit_type {
            self.install_unit_type(&module, unit_type)?;
        }

        for internal_enum in &module.internal_enums {
            self.install_internal_enum(module, internal_enum)?;
        }

        for (key, inst) in &module.associated_functions {
            self.install_associated_function(
                key.value_type,
                key.hash,
                inst,
                key.kind.into_hash_fn(),
            )?;
        }

        Ok(())
    }

    /// Install the given meta.
    fn install_meta(&mut self, item: Item, meta: Meta) -> Result<(), ContextError> {
        if let Some(existing) = self.meta.insert(item.clone(), meta.clone()) {
            return Err(ContextError::ConflictingMeta {
                item,
                existing: Box::new(existing),
                current: Box::new(meta),
            });
        }

        Ok(())
    }

    /// Install a single type.
    fn install_type(
        &mut self,
        module: &Module,
        value_type: Type,
        ty: &ModuleType,
    ) -> Result<(), ContextError> {
        let name = module.path.join(&ty.name);
        let hash = Hash::type_hash(&name);

        self.install_type_info(
            hash,
            ContextTypeInfo {
                type_check: TypeCheck::Type(value_type.as_type_hash()),
                name: name.clone(),
                value_type,
                type_info: ty.type_info,
            },
        )?;

        self.install_meta(
            name.clone(),
            Meta::Struct {
                value_type,
                object: MetaStruct {
                    item: name.clone(),
                    fields: None,
                },
            },
        )?;

        Ok(())
    }

    fn install_type_info(&mut self, hash: Hash, info: ContextTypeInfo) -> Result<(), ContextError> {
        self.names.insert(&info.name);

        // reverse lookup for types.
        if let Some(existing) = self.types_rev.insert(info.value_type, hash) {
            return Err(ContextError::ConflictingTypeHash {
                hash,
                existing,
                value_type: info.value_type,
            });
        }

        if let Some(existing) = self.types.insert(hash, info) {
            return Err(ContextError::ConflictingType {
                name: existing.name,
                existing: existing.type_info,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_function(
        &mut self,
        module: &Module,
        name: &Item,
        f: &ModuleFn,
    ) -> Result<(), ContextError> {
        let name = module.path.join(name);
        self.names.insert(&name);

        let hash = Hash::type_hash(&name);

        let signature = ContextSignature::Function {
            path: name.clone(),
            args: f.args,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.functions.insert(hash, f.handler.clone());

        self.meta.insert(
            name.clone(),
            Meta::Function {
                value_type: Type::Hash(hash),
                item: name.clone(),
            },
        );

        Ok(())
    }

    fn install_associated_function(
        &mut self,
        value_type: Type,
        hash: Hash,
        assoc: &ModuleAssociatedFn,
        hash_fn: impl FnOnce(Type, Hash) -> Hash,
    ) -> Result<(), ContextError> {
        let info = match self
            .types_rev
            .get(&value_type)
            .and_then(|hash| self.types.get(&hash))
        {
            Some(info) => info,
            None => {
                return Err(ContextError::MissingInstance {
                    instance_type: assoc.type_info,
                });
            }
        };

        let hash = hash_fn(value_type, hash);

        let signature = ContextSignature::Instance {
            path: info.name.clone(),
            name: assoc.name.clone(),
            args: assoc.args,
            self_type_info: info.type_info,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.functions.insert(hash, assoc.handler.clone());
        Ok(())
    }

    /// Install unit type.
    fn install_unit_type(
        &mut self,
        module: &Module,
        unit_type: &ModuleUnitType,
    ) -> Result<(), ContextError> {
        if self.unit_type.is_some() {
            return Err(ContextError::UnitAlreadyPresent);
        }

        let item = module.path.join(&unit_type.item);
        let hash = Hash::type_hash(&item);
        self.unit_type = Some(Hash::type_hash(&item));
        self.add_internal_tuple(None, item.clone(), 0, || ())?;

        self.install_type_info(
            hash,
            ContextTypeInfo {
                type_check: TypeCheck::Unit,
                name: item.clone(),
                value_type: Type::StaticType(crate::UNIT_TYPE),
                type_info: TypeInfo::StaticType(crate::UNIT_TYPE),
            },
        )?;

        Ok(())
    }

    /// Install generator state types.
    fn install_internal_enum(
        &mut self,
        module: &Module,
        internal_enum: &ModuleInternalEnum,
    ) -> Result<(), ContextError> {
        if !self.internal_enums.insert(internal_enum.static_type) {
            return Err(ContextError::InternalAlreadyPresent {
                name: internal_enum.name,
            });
        }

        let enum_item = module.path.join(&internal_enum.base_type);
        let enum_hash = Hash::type_hash(&enum_item);

        self.install_meta(
            enum_item.clone(),
            Meta::Enum {
                value_type: Type::StaticType(internal_enum.static_type),
                item: enum_item.clone(),
            },
        )?;

        self.install_type_info(
            enum_hash,
            ContextTypeInfo {
                type_check: TypeCheck::Type(internal_enum.static_type.hash),
                name: enum_item.clone(),
                value_type: Type::StaticType(internal_enum.static_type),
                type_info: TypeInfo::StaticType(internal_enum.static_type),
            },
        )?;

        for variant in &internal_enum.variants {
            let item = enum_item.clone().extended(variant.name);
            let hash = Hash::type_hash(&item);

            self.install_type_info(
                hash,
                ContextTypeInfo {
                    type_check: variant.type_check,
                    name: item.clone(),
                    value_type: Type::Hash(hash),
                    type_info: TypeInfo::StaticType(internal_enum.static_type),
                },
            )?;

            let tuple = MetaTuple {
                item: item.clone(),
                args: variant.args,
                hash,
            };

            let meta = Meta::VariantTuple {
                value_type: variant.value_type,
                enum_item: enum_item.clone(),
                tuple,
            };

            self.install_meta(item.clone(), meta)?;

            let signature = ContextSignature::Function {
                path: item,
                args: Some(variant.args),
            };

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, variant.constructor.clone());
        }

        Ok(())
    }

    /// Add a piece of internal tuple meta.
    fn add_internal_tuple<C, Args>(
        &mut self,
        enum_item: Option<Item>,
        item: Item,
        args: usize,
        constructor: C,
    ) -> Result<(), ContextError>
    where
        C: crate::module::Function<Args>,
        C::Return: ValueType,
    {
        let value_type = <C::Return as ValueType>::value_type();
        let hash = Hash::type_hash(&item);

        let tuple = MetaTuple {
            item: item.clone(),
            args,
            hash,
        };

        let meta = match enum_item {
            Some(enum_item) => Meta::VariantTuple {
                value_type,
                enum_item,
                tuple,
            },
            None => Meta::Tuple { value_type, tuple },
        };

        self.install_meta(item.clone(), meta)?;

        let constructor: Arc<Handler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        let signature = ContextSignature::Function {
            path: item,
            args: Some(args),
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.functions.insert(hash, constructor);
        Ok(())
    }

    /// Lookup the given function.
    pub(crate) fn lookup(&self, hash: Hash) -> Option<&Arc<Handler>> {
        self.functions.get(&hash)
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Context")
    }
}
