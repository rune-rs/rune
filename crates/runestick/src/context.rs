use crate::{
    collections::{HashMap, HashSet},
    module::{
        ModuleAssociatedFn, ModuleFn, ModuleInternalEnum, ModuleMacro, ModuleType, ModuleUnitType,
    },
    CompileMeta, CompileMetaKind, CompileMetaStruct, CompileMetaTuple, ComponentRef, ConstValue,
    Hash, IntoComponent, Item, Module, Names, Protocol, RuntimeContext, Stack, StaticType,
    TypeCheck, TypeInfo, TypeOf, VmError,
};
use std::{any, fmt, sync::Arc};

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
    #[error("conflicting meta {existing} while trying to insert {current}")]
    ConflictingMeta {
        /// The current meta we tried to insert.
        current: Box<CompileMeta>,
        /// The existing meta item.
        existing: Box<CompileMeta>,
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
    /// Error raised when attempting to register a conflicting constant.
    #[error("constant with name `{name}` already exists")]
    ConflictingConstantName {
        /// The name of the conflicting constant.
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
    #[error("module `{item}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        item: Item,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
    /// Raised when we try to register a conflicting type.
    #[error("type `{item}` already exists `{existing}`")]
    ConflictingType {
        /// The name we tried to register.
        item: Item,
        /// The type information for the type that already existed.
        existing: TypeInfo,
    },
    /// Raised when we try to register a conflicting type hash.
    #[error("tried to insert conflicting hash `{hash}` for `{existing}`")]
    ConflictingTypeHash {
        /// The hash we are trying to insert.
        hash: Hash,
        /// The hash that already existed.
        existing: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("variant with `{item}` already exists")]
    ConflictingVariant {
        /// The name of the conflicting variant.
        item: Item,
    },
    /// Error raised when attempting to register an instance function on an
    /// instance which does not exist.
    #[error("instance `{instance_type}` does not exist in module")]
    MissingInstance {
        /// The instance type.
        instance_type: TypeInfo,
    },
    /// Error raised when attempting to create a constant value.
    #[error("error when converting to constant value: {error}")]
    ValueError {
        /// The inner error.
        error: VmError,
    },
}

/// A function handler.
pub(crate) type Handler = dyn Fn(&mut Stack, usize) -> Result<(), VmError> + Send + Sync;

/// A (type erased) macro handler.
pub(crate) type Macro =
    dyn Fn(&dyn any::Any) -> Result<Box<dyn any::Any>, crate::Error> + Send + Sync;

/// Information on a specific type.
#[derive(Debug, Clone)]
pub struct ContextTypeInfo {
    /// The type check used for the current type.
    ///
    /// If absent, the type cannot be type checked for.
    pub type_check: TypeCheck,
    /// The name of the type.
    pub item: Item,
    /// The value type of the type.
    pub type_hash: Hash,
    /// Information on the type.
    pub type_info: TypeInfo,
}

impl fmt::Display for ContextTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.item, self.type_info)?;
        Ok(())
    }
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub enum ContextSignature {
    /// An unbound or static function
    Function {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the function.
        item: Item,
        /// Arguments.
        args: Option<usize>,
    },
    /// An instance function or method
    Instance {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the instance function.
        item: Item,
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
            Self::Function { item, args, .. } => {
                write!(fmt, "{}(", item)?;

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
                item,
                name,
                self_type_info,
                args,
                ..
            } => {
                write!(fmt, "{}::{}(self: {}", item, name, self_type_info)?;

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
    has_default_modules: bool,
    /// Item metadata in the context.
    meta: HashMap<Item, CompileMeta>,
    /// Registered native function handlers.
    functions: HashMap<Hash, Arc<Handler>>,
    /// Registered native macro handlers.
    macros: HashMap<Hash, Arc<Macro>>,
    /// Information on functions.
    functions_info: HashMap<Hash, ContextSignature>,
    /// Registered types.
    types: HashMap<Hash, ContextTypeInfo>,
    /// Reverse lookup for types.
    types_rev: HashMap<Hash, Hash>,
    /// Specialized information on unit types, if available.
    unit_type: Option<Hash>,
    /// Registered internal enums.
    internal_enums: HashSet<&'static StaticType>,
    /// All available names in the context.
    names: Names,
    /// Registered crates.
    crates: HashSet<Box<str>>,
    /// Constants visible in this context
    constants: HashMap<Hash, ConstValue>,
}

impl Context {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a default set of modules with the given configuration.
    ///
    /// * `stdio` determines if we include I/O functions that interact with
    ///   stdout and stderr by default, like `dbg`, `print`, and `println`.
    pub fn with_config(stdio: bool) -> Result<Self, ContextError> {
        let mut this = Self::new();
        this.install(&crate::modules::any::module()?)?;
        this.install(&crate::modules::bytes::module()?)?;
        this.install(&crate::modules::char::module()?)?;
        this.install(&crate::modules::cmp::module()?)?;
        this.install(&crate::modules::collections::module()?)?;
        this.install(&crate::modules::core::module()?)?;
        this.install(&crate::modules::float::module()?)?;
        this.install(&crate::modules::fmt::module()?)?;
        this.install(&crate::modules::future::module()?)?;
        this.install(&crate::modules::generator::module()?)?;
        this.install(&crate::modules::int::module()?)?;
        this.install(&crate::modules::io::module(stdio)?)?;
        this.install(&crate::modules::iter::module()?)?;
        this.install(&crate::modules::mem::module()?)?;
        this.install(&crate::modules::object::module()?)?;
        this.install(&crate::modules::ops::module()?)?;
        this.install(&crate::modules::option::module()?)?;
        this.install(&crate::modules::result::module()?)?;
        this.install(&crate::modules::stream::module()?)?;
        this.install(&crate::modules::string::module()?)?;
        this.install(&crate::modules::vec::module()?)?;
        this.has_default_modules = true;
        Ok(this)
    }

    /// Construct a runtime context used when executing the virtual machine.
    ///
    /// ```rust
    /// use runestick::{Context, Vm, Unit};
    /// use std::sync::Arc;
    ///
    /// # fn main() -> runestick::Result<()> {
    /// let context = Context::with_default_modules()?;
    ///
    /// let runtime = Arc::new(context.runtime());
    /// let unit = Arc::new(Unit::default());
    ///
    /// let vm = Vm::new(runtime, unit);
    /// # Ok(()) }
    /// ```
    pub fn runtime(&self) -> RuntimeContext {
        RuntimeContext {
            functions: self.functions.clone(),
            types: self.types.iter().map(|(k, t)| (*k, t.type_check)).collect(),
            constants: self.constants.clone(),
        }
    }

    /// Use the specified type check.
    pub fn type_check_for(&self, item: &Item) -> Option<TypeCheck> {
        let ty = self.types.get(&Hash::type_hash(item))?;
        Some(ty.type_check)
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        Self::with_config(true)
    }

    /// Check if context contains the given crate.
    pub fn contains_crate(&self, name: &str) -> bool {
        self.crates.contains(name)
    }

    /// Test if the context has the default modules installed.
    ///
    /// This determines among other things whether a prelude should be used or
    /// not.
    pub fn has_default_modules(&self) -> bool {
        self.has_default_modules
    }

    /// Iterate over known child components of the given name.
    pub fn iter_components<'a, I: 'a>(
        &'a self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'a>> + 'a
    where
        I: IntoIterator,
        I::Item: IntoComponent,
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

    /// Lookup the given native function handler in the context.
    pub fn lookup(&self, hash: Hash) -> Option<&Arc<Handler>> {
        self.functions.get(&hash)
    }

    /// Lookup the given macro handler.
    pub fn lookup_macro(&self, hash: Hash) -> Option<&Arc<Macro>> {
        self.macros.get(&hash)
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(&self, name: &Item) -> Option<CompileMeta> {
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
        if let Some(ComponentRef::Crate(name)) = module.item.first() {
            self.crates.insert(name.into());
        }

        for (type_hash, ty) in &module.types {
            self.install_type(module, *type_hash, ty)?;
        }

        for (name, f) in &module.functions {
            self.install_function(module, name, f)?;
        }

        for (name, m) in &module.macros {
            self.install_macro(module, name, m)?;
        }

        for (name, m) in &module.constants {
            self.install_constant(module, name, m)?;
        }

        if let Some(unit_type) = &module.unit_type {
            self.install_unit_type(module, unit_type)?;
        }

        for internal_enum in &module.internal_enums {
            self.install_internal_enum(module, internal_enum)?;
        }

        for (key, inst) in &module.associated_functions {
            self.install_associated_function(
                key.type_hash,
                key.hash,
                inst,
                |instance_type, field| key.kind.hash(instance_type, field),
            )?;
        }

        Ok(())
    }

    /// Install the given meta.
    fn install_meta(&mut self, meta: CompileMeta) -> Result<(), ContextError> {
        if let Some(existing) = self.meta.insert(meta.item.item.clone(), meta.clone()) {
            return Err(ContextError::ConflictingMeta {
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
        type_hash: Hash,
        ty: &ModuleType,
    ) -> Result<(), ContextError> {
        let item = module.item.extended(&*ty.name);
        let hash = Hash::type_hash(&item);

        self.install_type_info(
            hash,
            ContextTypeInfo {
                type_check: TypeCheck::Type(type_hash),
                item: item.clone(),
                type_hash,
                type_info: ty.type_info.clone(),
            },
        )?;

        self.install_meta(CompileMeta {
            item: Arc::new(item.into()),
            kind: CompileMetaKind::Struct {
                type_hash,
                object: CompileMetaStruct {
                    fields: Default::default(),
                },
            },
            source: None,
        })?;

        Ok(())
    }

    fn install_type_info(&mut self, hash: Hash, info: ContextTypeInfo) -> Result<(), ContextError> {
        self.names.insert(&info.item);

        // reverse lookup for types.
        if let Some(existing) = self.types_rev.insert(info.type_hash, hash) {
            return Err(ContextError::ConflictingTypeHash { hash, existing });
        }

        self.constants.insert(
            Hash::instance_function(info.type_hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(info.item.to_string()),
        );

        if let Some(existing) = self.types.insert(hash, info) {
            return Err(ContextError::ConflictingType {
                item: existing.item,
                existing: existing.type_info,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_function(
        &mut self,
        module: &Module,
        item: &Item,
        f: &ModuleFn,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);
        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        let signature = ContextSignature::Function {
            type_hash: hash,
            item: item.clone(),
            args: f.args,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        self.functions.insert(hash, f.handler.clone());
        self.meta.insert(
            item.clone(),
            CompileMeta {
                item: Arc::new(item.into()),
                kind: CompileMetaKind::Function {
                    type_hash: hash,
                    is_test: false,
                },
                source: None,
            },
        );

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_macro(
        &mut self,
        module: &Module,
        item: &Item,
        m: &ModuleMacro,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);

        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.macros.insert(hash, m.handler.clone());
        Ok(())
    }

    /// Install a constant and check for duplicates.
    fn install_constant(
        &mut self,
        module: &Module,
        item: &Item,
        v: &ConstValue,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);

        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.constants.insert(hash, v.clone());

        self.meta.insert(
            item.clone(),
            CompileMeta {
                item: Arc::new(item.into()),
                kind: CompileMetaKind::Const {
                    const_value: v.clone(),
                },
                source: None,
            },
        );
        Ok(())
    }

    fn install_associated_function(
        &mut self,
        type_hash: Hash,
        hash: Hash,
        assoc: &ModuleAssociatedFn,
        hash_fn: impl FnOnce(Hash, Hash) -> Hash,
    ) -> Result<(), ContextError> {
        let info = match self
            .types_rev
            .get(&type_hash)
            .and_then(|hash| self.types.get(hash))
        {
            Some(info) => info,
            None => {
                return Err(ContextError::MissingInstance {
                    instance_type: assoc.type_info.clone(),
                });
            }
        };

        let hash = hash_fn(type_hash, hash);

        let signature = ContextSignature::Instance {
            type_hash,
            item: info.item.clone(),
            name: assoc.name.clone(),
            args: assoc.args,
            self_type_info: info.type_info.clone(),
        };
        let item = info.item.extended(&assoc.name);

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }
        self.meta.insert(
            item.clone(),
            CompileMeta {
                item: Arc::new(item.into()),
                kind: CompileMetaKind::Function {
                    type_hash: hash,
                    is_test: false,
                },
                source: None,
            },
        );

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

        let item = module.item.extended(&*unit_type.name);
        let hash = Hash::type_hash(&item);
        self.unit_type = Some(Hash::type_hash(&item));
        self.add_internal_tuple(None, item.clone(), 0, || ())?;

        self.install_type_info(
            hash,
            ContextTypeInfo {
                type_check: TypeCheck::Unit,
                item,
                type_hash: crate::UNIT_TYPE.hash,
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

        let enum_item = module.item.join(&internal_enum.base_type);
        let enum_hash = Hash::type_hash(&enum_item);

        self.install_meta(CompileMeta {
            item: Arc::new(enum_item.clone().into()),
            kind: CompileMetaKind::Enum {
                type_hash: internal_enum.static_type.hash,
            },
            source: None,
        })?;

        self.install_type_info(
            enum_hash,
            ContextTypeInfo {
                type_check: TypeCheck::Type(internal_enum.static_type.hash),
                item: enum_item.clone(),
                type_hash: internal_enum.static_type.hash,
                type_info: TypeInfo::StaticType(internal_enum.static_type),
            },
        )?;

        for variant in &internal_enum.variants {
            let item = enum_item.extended(variant.name);
            let hash = Hash::type_hash(&item);

            self.install_type_info(
                hash,
                ContextTypeInfo {
                    type_check: variant.type_check,
                    item: item.clone(),
                    type_hash: hash,
                    type_info: TypeInfo::StaticType(internal_enum.static_type),
                },
            )?;

            self.install_meta(CompileMeta {
                item: Arc::new(item.clone().into()),
                kind: CompileMetaKind::TupleVariant {
                    type_hash: variant.type_hash,
                    enum_item: enum_item.clone(),
                    tuple: CompileMetaTuple {
                        args: variant.args,
                        hash,
                    },
                },
                source: None,
            })?;

            let signature = ContextSignature::Function {
                type_hash: variant.type_hash,
                item,
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
        C::Return: TypeOf,
    {
        let type_hash = <C::Return as TypeOf>::type_hash();
        let hash = Hash::type_hash(&item);

        let tuple = CompileMetaTuple { args, hash };

        let meta = match enum_item {
            Some(enum_item) => CompileMeta {
                item: Arc::new(item.clone().into()),
                kind: CompileMetaKind::TupleVariant {
                    type_hash,
                    enum_item,
                    tuple,
                },
                source: None,
            },
            None => CompileMeta {
                item: Arc::new(item.clone().into()),
                kind: CompileMetaKind::TupleStruct { type_hash, tuple },
                source: None,
            },
        };

        self.install_meta(meta)?;

        let constructor: Arc<Handler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.constants.insert(
            Hash::instance_function(type_hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        let signature = ContextSignature::Function {
            type_hash,
            item,
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
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Context")
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send, Sync);
