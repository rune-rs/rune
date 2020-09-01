use crate::collections::HashMap;
use crate::module::{
    ModuleInstanceFunction, ModuleOptionTypes, ModuleResultTypes, ModuleType, ModuleUnitType,
};
use crate::{
    Hash, IntoTypeHash, Item, Meta, MetaStruct, MetaTuple, Module, OptionVariant, ReflectValueType,
    ResultVariant, Stack, TypeCheck, Value, ValueType, ValueTypeInfo, VmError,
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
    /// Conflicting `Option` types.
    #[error("`Option` types are already present")]
    OptionAlreadyPresent,
    /// Conflicting `Result` types.
    #[error("`Result` types are already present")]
    ResultAlreadyPresent,
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
        signature: FnSignature,
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
    #[error("instance function `{name}` for type `{value_type_info}` already exists")]
    ConflictingInstanceFunction {
        /// Type that we register the instance function for.
        value_type_info: ValueTypeInfo,
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
        existing: ValueTypeInfo,
    },
    /// Raised when we try to register a conflicting type hash.
    #[error("tried to insert conflicting hash type `{hash}` (existing `{existing}`) for type `{value_type}`")]
    ConflictingTypeHash {
        /// The hash we are trying to insert.
        hash: Hash,
        /// The hash that already existed.
        existing: Hash,
        /// The type we're trying to insert.
        value_type: ValueType,
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
        instance_type: ValueTypeInfo,
    },
    /// Error raised when attempting to register a type that doesn't have a type
    /// hash into a context.
    #[error("type `{value_type}` cannot be defined dynamically")]
    UnsupportedValueType {
        /// The type we tried to register.
        value_type: ValueType,
    },
}

/// A function handler.
pub(crate) type Handler = dyn Fn(&mut Stack, usize) -> Result<(), VmError> + Sync;

/// Information on a specific type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// The type check used for the current type.
    ///
    /// If absent, the type cannot be type checked for.
    pub type_check: TypeCheck,
    /// The name of the type.
    pub name: Item,
    /// The value type of the type.
    pub value_type: ValueType,
    /// Information on the type.
    pub value_type_info: ValueTypeInfo,
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.name, self.value_type_info)?;
        Ok(())
    }
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub enum FnSignature {
    Free {
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
        self_type_info: ValueTypeInfo,
    },
}

impl FnSignature {
    /// Construct a new global function signature.
    pub fn new_free(path: Item, args: Option<usize>) -> Self {
        Self::Free { path, args }
    }

    /// Construct a new function signature.
    pub fn new_inst(
        path: Item,
        name: String,
        args: Option<usize>,
        self_type_info: ValueTypeInfo,
    ) -> Self {
        Self::Instance {
            path,
            name,
            args,
            self_type_info,
        }
    }
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Free { path, args } => {
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
    /// Item metadata in the context.
    meta: HashMap<Item, Meta>,
    /// Free functions.
    functions: HashMap<Hash, Arc<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
    /// Registered types.
    types: HashMap<Hash, TypeInfo>,
    /// Reverse lookup for types.
    types_rev: HashMap<ValueType, Hash>,
    /// Specialized information on unit types, if available.
    unit_type: Option<Hash>,
    /// Specialized information on `Result` types, if available.
    result_type: Option<Hash>,
    /// Specialized information on `Option` types, if available.
    option_type: Option<Hash>,
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
    pub fn with_default_packages() -> Result<Self, ContextError> {
        let mut this = Self::new();
        this.install(&crate::packages::core::module()?)?;
        this.install(&crate::packages::bytes::module()?)?;
        this.install(&crate::packages::string::module()?)?;
        this.install(&crate::packages::int::module()?)?;
        this.install(&crate::packages::float::module()?)?;
        this.install(&crate::packages::test::module()?)?;
        this.install(&crate::packages::iter::module()?)?;
        this.install(&crate::packages::vec::module()?)?;
        this.install(&crate::packages::object::module()?)?;
        this.install(&crate::packages::result::module()?)?;
        this.install(&crate::packages::option::module()?)?;
        this.install(&crate::packages::future::module()?)?;
        Ok(this)
    }

    /// Access the currently known unit type.
    pub fn unit_type(&self) -> Option<Hash> {
        self.unit_type
    }

    /// Access the currently known result type.
    pub fn result_type(&self) -> Option<Hash> {
        self.result_type
    }

    /// Access the currently known option type.
    pub fn option_type(&self) -> Option<Hash> {
        self.option_type
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(&self, name: &Item) -> Option<Meta> {
        self.meta.get(name).cloned()
    }

    /// Iterate over all available functions
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &FnSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Iterate over all available types.
    pub fn iter_types(&self) -> impl Iterator<Item = (Hash, &TypeInfo)> {
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

        for (name, (handler, args)) in &module.functions {
            self.install_function(&module, name, handler, args)?;
        }

        if let Some(unit_type) = &module.unit_type {
            self.install_unit_type(&module, unit_type)?;
        }

        if let Some(result_types) = &module.result_types {
            self.install_result_types(&module, result_types)?;
        }

        if let Some(option_types) = &module.option_types {
            self.install_option_types(&module, option_types)?;
        }

        for ((value_type, hash), inst) in &module.instance_functions {
            self.install_module_instance_function(*value_type, *hash, inst)?;
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
        value_type: ValueType,
        ty: &ModuleType,
    ) -> Result<(), ContextError> {
        let name = module.path.join(&ty.name);
        let hash = Hash::type_hash(&name);

        self.install_type_info(
            hash,
            TypeInfo {
                type_check: TypeCheck::Type(value_type.as_type_hash()),
                name: name.clone(),
                value_type,
                value_type_info: ty.value_type_info,
            },
        )?;

        self.install_meta(
            name.clone(),
            Meta::MetaStruct {
                value_type,
                object: MetaStruct {
                    item: name.clone(),
                    fields: None,
                },
            },
        )?;

        Ok(())
    }

    fn install_type_info(&mut self, hash: Hash, type_info: TypeInfo) -> Result<(), ContextError> {
        let value_type = type_info.value_type;

        if let Some(existing) = self.types.insert(hash, type_info) {
            return Err(ContextError::ConflictingType {
                name: existing.name,
                existing: existing.value_type_info,
            });
        }

        // reverse lookup for types.
        if let Some(existing) = self.types_rev.insert(value_type, hash) {
            return Err(ContextError::ConflictingTypeHash {
                hash,
                existing,
                value_type,
            });
        }

        Ok(())
    }

    fn install_variant_type_info(
        &mut self,
        hash: Hash,
        type_info: TypeInfo,
    ) -> Result<(), ContextError> {
        if let Some(existing) = self.types.insert(hash, type_info) {
            return Err(ContextError::ConflictingType {
                name: existing.name,
                existing: existing.value_type_info,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_function(
        &mut self,
        module: &Module,
        name: &Item,
        handler: &Arc<Handler>,
        args: &Option<usize>,
    ) -> Result<(), ContextError> {
        let name = module.path.join(name);
        let hash = Hash::type_hash(&name);
        let signature = FnSignature::new_free(name.clone(), *args);

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.functions.insert(hash, handler.clone());

        self.meta.insert(
            name.clone(),
            Meta::MetaFunction {
                value_type: ValueType::Type(hash),
                item: name.clone(),
            },
        );

        Ok(())
    }

    fn install_module_instance_function(
        &mut self,
        value_type: ValueType,
        hash: Hash,
        inst: &ModuleInstanceFunction,
    ) -> Result<(), ContextError> {
        let type_info = match self
            .types_rev
            .get(&value_type)
            .and_then(|hash| self.types.get(&hash))
        {
            Some(type_info) => type_info,
            None => {
                return Err(ContextError::MissingInstance {
                    instance_type: inst.value_type_info,
                });
            }
        };

        let hash = Hash::instance_function(value_type, hash);

        let signature = FnSignature::new_inst(
            type_info.name.clone(),
            inst.name.clone(),
            inst.args,
            type_info.value_type_info,
        );

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: old,
                hash,
            });
        }

        self.functions.insert(hash, inst.handler.clone());
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
            TypeInfo {
                type_check: TypeCheck::Unit,
                name: item.clone(),
                value_type: ValueType::StaticType(crate::UNIT_TYPE),
                value_type_info: ValueTypeInfo::StaticType(crate::UNIT_TYPE),
            },
        )?;

        Ok(())
    }

    /// Install option types.
    fn install_result_types(
        &mut self,
        module: &Module,
        result_types: &ModuleResultTypes,
    ) -> Result<(), ContextError> {
        if self.result_type.is_some() {
            return Err(ContextError::ResultAlreadyPresent);
        }

        let result_item = module.path.join(&result_types.result_type);
        let ok_item = module.path.join(&result_types.ok_type);
        let err_item = module.path.join(&result_types.err_type);

        self.install_meta(
            result_item.clone(),
            Meta::MetaEnum {
                value_type: ValueType::StaticType(crate::RESULT_TYPE),
                item: result_item.clone(),
            },
        )?;

        let enum_type = Hash::type_hash(&result_item);
        self.result_type = Some(enum_type);

        self.add_internal_tuple(
            Some(result_item.clone()),
            ok_item.clone(),
            1,
            Ok::<Value, Value>,
        )?;
        self.add_internal_tuple(
            Some(result_item.clone()),
            err_item.clone(),
            1,
            Err::<Value, Value>,
        )?;

        self.install_type_info(
            enum_type,
            TypeInfo {
                type_check: TypeCheck::Type(enum_type),
                name: result_item,
                value_type: ValueType::StaticType(crate::RESULT_TYPE),
                value_type_info: ValueTypeInfo::StaticType(crate::RESULT_TYPE),
            },
        )?;

        let hash = Hash::type_hash(&ok_item);

        self.install_variant_type_info(
            hash,
            TypeInfo {
                type_check: TypeCheck::Result(ResultVariant::Ok),
                name: ok_item,
                value_type: ValueType::Type(hash),
                value_type_info: ValueTypeInfo::StaticType(crate::RESULT_TYPE),
            },
        )?;

        let hash = Hash::type_hash(&err_item);

        self.install_variant_type_info(
            hash,
            TypeInfo {
                type_check: TypeCheck::Result(ResultVariant::Err),
                name: err_item,
                value_type: ValueType::Type(hash),
                value_type_info: ValueTypeInfo::StaticType(crate::RESULT_TYPE),
            },
        )?;

        Ok(())
    }

    /// Install option types.
    fn install_option_types(
        &mut self,
        module: &Module,
        option_types: &ModuleOptionTypes,
    ) -> Result<(), ContextError> {
        if self.option_type.is_some() {
            return Err(ContextError::OptionAlreadyPresent);
        }

        let option_item = module.path.join(&option_types.option_type);
        let some_item = module.path.join(&option_types.some_type);
        let none_item = module.path.join(&option_types.none_type);

        self.install_meta(
            option_item.clone(),
            Meta::MetaEnum {
                value_type: ValueType::StaticType(crate::OPTION_TYPE),
                item: option_item.clone(),
            },
        )?;

        let enum_hash = Hash::type_hash(&option_item);

        self.option_type = Some(enum_hash);

        self.add_internal_tuple(
            Some(option_item.clone()),
            some_item.clone(),
            1,
            Some::<Value>,
        )?;
        self.add_internal_tuple(Some(option_item.clone()), none_item.clone(), 0, || {
            None::<Value>
        })?;

        self.install_type_info(
            enum_hash,
            TypeInfo {
                type_check: TypeCheck::Type(enum_hash),
                name: option_item,
                value_type: ValueType::StaticType(crate::OPTION_TYPE),
                value_type_info: ValueTypeInfo::StaticType(crate::OPTION_TYPE),
            },
        )?;

        let hash = Hash::type_hash(&some_item);

        self.install_variant_type_info(
            hash,
            TypeInfo {
                type_check: TypeCheck::Option(OptionVariant::Some),
                name: some_item,
                value_type: ValueType::Type(hash),
                value_type_info: ValueTypeInfo::StaticType(crate::OPTION_TYPE),
            },
        )?;

        let hash = Hash::type_hash(&none_item);

        self.install_variant_type_info(
            hash,
            TypeInfo {
                type_check: TypeCheck::Option(OptionVariant::None),
                name: none_item,
                value_type: ValueType::Type(hash),
                value_type_info: ValueTypeInfo::StaticType(crate::OPTION_TYPE),
            },
        )?;

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
        C: crate::Function<Args>,
        C::Return: ReflectValueType,
    {
        let value_type = <C::Return as ReflectValueType>::value_type();
        let hash = Hash::type_hash(&item);

        let tuple = MetaTuple {
            item: item.clone(),
            args,
            hash,
        };

        let meta = match enum_item {
            Some(enum_item) => Meta::MetaVariantTuple {
                value_type,
                enum_item,
                tuple,
            },
            None => Meta::MetaTuple { value_type, tuple },
        };

        self.install_meta(item.clone(), meta)?;

        let constructor: Arc<Handler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));
        let signature = FnSignature::new_free(item, Some(args));

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

/// Trait used to determine what can be used as an instance function name.
pub trait IntoInstFnHash: Copy {
    /// Generate a locally unique hash to check for conflicts.
    fn to_hash(self) -> Hash;

    /// Get a human readable name for the function.
    fn to_name(self) -> String;
}

impl<'a> IntoInstFnHash for &'a str {
    fn to_hash(self) -> Hash {
        Hash::of(self)
    }

    fn to_name(self) -> String {
        self.to_owned()
    }
}

/// A built in instance function.
#[derive(Debug, Clone, Copy)]
pub struct BuiltInInstanceFn {
    name: &'static str,
    pub(crate) hash: Hash,
}

impl IntoInstFnHash for BuiltInInstanceFn {
    fn to_hash(self) -> Hash {
        self.hash
    }

    fn to_name(self) -> String {
        String::from(self.name)
    }
}

impl IntoTypeHash for BuiltInInstanceFn {
    fn into_type_hash(self) -> Hash {
        self.hash
    }
}

impl std::ops::Deref for BuiltInInstanceFn {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

/// The function to access an index.
pub const INDEX_GET: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "index_get",
    hash: Hash::new(0xadb5b27e2a4d2dec),
};

/// The function to set an index.
pub const INDEX_SET: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "index_set",
    hash: Hash::new(0x162943f7bd03ad36),
};

/// The function to implement for the addition operation.
pub const ADD: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "add",
    hash: Hash::new(0xe4ecf51fa0bf1076),
};

/// The function to implement for the addition assign operation.
pub const ADD_ASSIGN: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "add_assign",
    hash: Hash::new(0x42451ccb0a2071a9),
};

/// The function to implement for the subtraction operation.
pub const SUB: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "sub",
    hash: Hash::new(0x6fa86a5f18d0bf71),
};

/// The function to implement for the subtraction assign operation.
pub const SUB_ASSIGN: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "sub_assign",
    hash: Hash::new(0x5939bb56a1415284),
};

/// The function to implement for the multiply operation.
pub const MUL: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "mul",
    hash: Hash::new(0xb09e99dc94091d1c),
};

/// The function to implement for the multiply assign operation.
pub const MUL_ASSIGN: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "mul_assign",
    hash: Hash::new(0x29a54b727f980ebf),
};

/// The function to implement for the division operation.
pub const DIV: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "div",
    hash: Hash::new(0xf26d6eea1afca6e8),
};

/// The function to implement for the division assign operation.
pub const DIV_ASSIGN: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "div_assign",
    hash: Hash::new(0x4dd087a8281c04e6),
};

/// The function to implement for the modulo operation.
pub const REM: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "mod",
    hash: Hash::new(0x5c6293639c74e671),
};

/// Function used for a fmt::Display::fmt implementation.
pub const FMT_DISPLAY: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "fmt",
    hash: Hash::new(0x811b62957ea9d9f9),
};

/// Function used to convert an argument into an iterator.
pub const INTO_ITER: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "into_iter",
    hash: Hash::new(0x15a85c8d774b4065),
};

/// The function to call to continue iteration.
pub const NEXT: BuiltInInstanceFn = BuiltInInstanceFn {
    name: "next",
    hash: Hash::new(0xc3cde069de2ba320),
};
