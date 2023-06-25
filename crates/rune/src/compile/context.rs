use core::fmt;

use crate::no_std::collections::{BTreeSet, HashMap, HashSet};
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::meta;
#[cfg(feature = "doc")]
use crate::compile::Docs;
use crate::compile::{ComponentRef, ContextError, IntoComponent, Item, ItemBuf, MetaInfo, Names};
use crate::module::{
    Fields, Function, InternalEnum, Module, ModuleAssociated, ModuleAttributeMacro, ModuleConstant,
    ModuleFunction, ModuleMacro, ModuleType, TypeSpecification, UnitType,
};
use crate::runtime::{
    AttributeMacroHandler, ConstValue, FunctionHandler, MacroHandler, Protocol, RuntimeContext,
    StaticType, TypeCheck, TypeInfo, TypeOf, VariantRtti,
};
use crate::Hash;

/// Context metadata.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct ContextMeta {
    /// Type hash for the given meta item.
    pub(crate) hash: Hash,
    /// The item of the returned compile meta.
    pub(crate) item: Option<ItemBuf>,
    /// The kind of the compile meta.
    pub(crate) kind: meta::Kind,
    /// Documentation associated with a context meta.
    #[cfg(feature = "doc")]
    pub(crate) docs: Docs,
}

impl ContextMeta {
    pub(crate) fn info(&self) -> MetaInfo {
        MetaInfo::new(&self.kind, self.hash, self.item.as_deref())
    }
}

/// Information on a specific type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct ContextType {
    /// Item of the type.
    item: ItemBuf,
    /// Type hash.
    hash: Hash,
    /// The type check used for the current type.
    type_check: Option<TypeCheck>,
    /// Complete detailed information on the hash.
    type_info: TypeInfo,
    /// Type parameters.
    type_parameters: Hash,
}

impl fmt::Display for ContextType {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.item, self.type_info)?;
        Ok(())
    }
}

/// [Context] used for the Rune language.
///
/// See [Build::with_context][crate::Build::with_context].
///
/// At runtime this needs to be converted into a [RuntimeContext] when used with
/// a [Vm][crate::runtime::Vm]. This is done through [Context::runtime].
///
/// A [Context] contains:
/// * Native functions.
/// * Native instance functions.
/// * And native type definitions.
#[derive(Default)]
pub struct Context {
    /// Unique modules installed in the context.
    unique: HashSet<&'static str>,
    /// Whether or not to include the prelude when constructing a new unit.
    has_default_modules: bool,
    /// Registered metadata, in the order that it was registered.
    meta: Vec<ContextMeta>,
    /// Item metadata in the context.
    hash_to_meta: HashMap<Hash, Vec<usize>>,
    /// Store item to hash mapping.
    item_to_hash: HashMap<ItemBuf, BTreeSet<Hash>>,
    /// Registered native function handlers.
    functions: HashMap<Hash, Arc<FunctionHandler>>,
    /// Information on associated types.
    #[cfg(feature = "doc")]
    associated: HashMap<Hash, Vec<Hash>>,
    /// Registered native macro handlers.
    macros: HashMap<Hash, Arc<MacroHandler>>,
    /// Registered native attribute macro handlers.
    attribute_macros: HashMap<Hash, Arc<AttributeMacroHandler>>,
    /// Registered types.
    types: HashMap<Hash, ContextType>,
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
    /// Construct a new empty [Context].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [Context] containing the default set of modules with the
    /// given configuration.
    ///
    /// `stdio` determines if we include I/O functions that interact with stdout
    /// and stderr by default, like `dbg`, `print`, and `println`. If this is
    /// `false` all the corresponding low-level I/O functions have to be
    /// provided through a different module.
    ///
    /// These are:
    ///
    /// * `::std::io::dbg`
    /// * `::std::io::print`
    /// * `::std::io::println`
    pub fn with_config(#[allow(unused)] stdio: bool) -> Result<Self, ContextError> {
        let mut this = Self::new();
        // This must go first, because it includes types which are used in other modules.
        this.install(crate::modules::core::module()?)?;

        this.install(crate::modules::any::module()?)?;
        this.install(crate::modules::bytes::module()?)?;
        this.install(crate::modules::char::module()?)?;
        this.install(crate::modules::cmp::module()?)?;
        this.install(crate::modules::collections::module()?)?;
        this.install(crate::modules::f64::module()?)?;
        this.install(crate::modules::fmt::module()?)?;
        this.install(crate::modules::future::module()?)?;
        this.install(crate::modules::generator::module()?)?;
        this.install(crate::modules::i64::module()?)?;
        #[cfg(feature = "std")]
        this.install(crate::modules::io::module(stdio)?)?;
        this.install(crate::modules::iter::module()?)?;
        this.install(crate::modules::macros::module()?)?;
        this.install(crate::modules::mem::module()?)?;
        this.install(crate::modules::object::module()?)?;
        this.install(crate::modules::ops::module()?)?;
        this.install(crate::modules::option::module()?)?;
        this.install(crate::modules::result::module()?)?;
        this.install(crate::modules::stream::module()?)?;
        this.install(crate::modules::string::module()?)?;
        this.install(crate::modules::test::module()?)?;
        this.install(crate::modules::vec::module()?)?;
        this.has_default_modules = true;
        Ok(this)
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        Self::with_config(true)
    }

    /// Construct a runtime context used when executing the virtual machine.
    ///
    /// This is not a cheap operation, since it requires cloning things out of
    /// the build-time [Context] which are necessary at runtime.
    ///
    /// ```
    /// use rune::{Context, Vm, Unit};
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    ///
    /// let runtime = Arc::new(context.runtime());
    /// let unit = Arc::new(Unit::default());
    ///
    /// let vm = Vm::new(runtime, unit);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn runtime(&self) -> RuntimeContext {
        RuntimeContext::new(self.functions.clone(), self.constants.clone())
    }

    /// Install the specified module.
    ///
    /// This installs everything that has been declared in the given [Module]
    /// and ensures that they are compatible with the overall context, like
    /// ensuring that a given type is only declared once.
    pub fn install<M>(&mut self, module: M) -> Result<(), ContextError>
    where
        M: AsRef<Module>,
    {
        let module = module.as_ref();

        if let Some(id) = module.unique {
            if !self.unique.insert(id) {
                return Ok(());
            }
        }

        if let Some(ComponentRef::Crate(name)) = module.item.first() {
            self.crates.insert(name.into());
        }

        self.install_module(module)?;

        for ty in &module.types {
            self.install_type(module, ty)?;
        }

        for f in &module.functions {
            self.install_function(module, f)?;
        }

        for m in &module.macros {
            self.install_macro(module, m)?;
        }

        for m in &module.attribute_macros {
            self.install_attribute_macro(module, m)?;
        }

        for m in &module.constants {
            self.install_constant(module, m)?;
        }

        if let Some(unit_type) = &module.unit_type {
            self.install_unit_type(module, unit_type)?;
        }

        for internal_enum in &module.internal_enums {
            self.install_internal_enum(module, internal_enum)?;
        }

        for assoc in &module.associated {
            self.install_associated(assoc)?;
        }

        Ok(())
    }

    /// Iterate over all available functions in the [Context].
    #[cfg(any(feature = "cli", feature = "languageserver"))]
    pub(crate) fn iter_functions(&self) -> impl Iterator<Item = (&ContextMeta, &meta::Signature)> {
        self.meta.iter().flat_map(|meta| {
            let signature = meta.kind.as_signature()?;
            Some((meta, signature))
        })
    }

    /// Iterate over all available types in the [Context].
    #[cfg(feature = "cli")]
    pub(crate) fn iter_types(&self) -> impl Iterator<Item = (Hash, &Item)> {
        use core::iter;

        let mut it = self.types.iter();

        iter::from_fn(move || {
            let (hash, ty) = it.next()?;
            Some((*hash, ty.item.as_ref()))
        })
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<'a, I: 'a>(
        &'a self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'a>> + 'a
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.names.iter_components(iter)
    }

    /// Access the context meta for the given item.
    ///
    /// If this returns `Some`, at least one context meta is guaranteed to be
    /// available.
    pub(crate) fn lookup_meta(
        &self,
        item: &Item,
    ) -> Option<impl Iterator<Item = &ContextMeta> + Clone> {
        let hashes = self.item_to_hash.get(item)?;

        Some(hashes.iter().flat_map(|hash| {
            let indexes = self
                .hash_to_meta
                .get(hash)
                .map(Vec::as_slice)
                .unwrap_or_default();
            indexes.iter().map(|&i| &self.meta[i])
        }))
    }

    /// Lookup meta by its hash.
    #[cfg(feature = "doc")]
    pub(crate) fn lookup_meta_by_hash(
        &self,
        hash: Hash,
    ) -> impl ExactSizeIterator<Item = &ContextMeta> + Clone {
        let indexes = self
            .hash_to_meta
            .get(&hash)
            .map(Vec::as_slice)
            .unwrap_or_default();

        indexes.iter().map(|&i| &self.meta[i])
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.names.contains_prefix(item)
    }

    /// Lookup the given native function handler in the context.
    pub(crate) fn lookup_function(&self, hash: Hash) -> Option<&Arc<FunctionHandler>> {
        self.functions.get(&hash)
    }

    /// Get all associated types for the given hash.
    #[cfg(feature = "doc")]
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = Hash> + '_ {
        self.associated
            .get(&hash)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .copied()
    }

    /// Lookup the given macro handler.
    pub(crate) fn lookup_macro(&self, hash: Hash) -> Option<&Arc<MacroHandler>> {
        self.macros.get(&hash)
    }

    /// Lookup the given attribute macro handler.
    pub(crate) fn lookup_attribute_macro(&self, hash: Hash) -> Option<&Arc<AttributeMacroHandler>> {
        self.attribute_macros.get(&hash)
    }

    /// Look up the type check implementation for the specified type hash.
    pub(crate) fn type_check_for(&self, hash: Hash) -> Option<TypeCheck> {
        let ty = self.types.get(&hash)?;
        ty.type_check
    }

    /// Iterate over available crates.
    #[cfg(feature = "doc")]
    pub(crate) fn iter_crates(&self) -> impl Iterator<Item = &str> {
        self.crates.iter().map(|s| s.as_ref())
    }

    /// Check if context contains the given crate.
    pub(crate) fn contains_crate(&self, name: &str) -> bool {
        self.crates.contains(name)
    }

    /// Test if the context has the default modules installed.
    ///
    /// This determines among other things whether a prelude should be used or
    /// not.
    pub(crate) fn has_default_modules(&self) -> bool {
        self.has_default_modules
    }

    /// Install the given meta.
    fn install_meta(&mut self, meta: ContextMeta) -> Result<(), ContextError> {
        if let Some(item) = &meta.item {
            self.names.insert(item);

            self.item_to_hash
                .entry(item.clone())
                .or_default()
                .insert(meta.hash);
        }

        #[cfg(feature = "doc")]
        if let Some(h) = meta.kind.associated_container() {
            let assoc = self.associated.entry(h).or_default();
            assoc.push(meta.hash);
        }

        let hash = meta.hash;
        let index = self.meta.len();
        self.meta.push(meta);
        self.hash_to_meta.entry(hash).or_default().push(index);
        Ok(())
    }

    /// Install a module, ensuring that its meta is defined.
    fn install_module(&mut self, m: &Module) -> Result<(), ContextError> {
        self.names.insert(&m.item);

        let mut current = Some((m.item.as_ref(), Some(&m.docs)));

        #[allow(unused)]
        while let Some((item, docs)) = current.take() {
            self.install_meta(ContextMeta {
                hash: Hash::type_hash(item),
                item: Some(item.to_owned()),
                kind: meta::Kind::Module,
                #[cfg(feature = "doc")]
                docs: docs.cloned().unwrap_or_default(),
            })?;

            current = item.parent().map(|item| (item, None));
        }

        Ok(())
    }

    /// Install a single type.
    fn install_type(&mut self, module: &Module, ty: &ModuleType) -> Result<(), ContextError> {
        let item = module.item.join(&ty.item);

        self.install_type_info(ContextType {
            item: item.clone(),
            hash: ty.hash,
            type_check: None,
            type_info: ty.type_info.clone(),
            type_parameters: ty.type_parameters,
        })?;

        let parameters = Hash::EMPTY.with_type_parameters(ty.type_parameters);

        let kind = if let Some(spec) = &ty.spec {
            match spec {
                TypeSpecification::Struct(fields) => meta::Kind::Struct {
                    fields: match fields {
                        Fields::Named(fields) => meta::Fields::Named(meta::FieldsNamed {
                            fields: fields.iter().copied().map(Box::<str>::from).collect(),
                        }),
                        Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                        Fields::Empty => meta::Fields::Empty,
                    },
                    constructor: None,
                    parameters,
                },
                TypeSpecification::Enum(en) => {
                    for (index, variant) in en.variants.iter().enumerate() {
                        let Some(fields) = &variant.fields else {
                            continue;
                        };

                        let item = item.extended(variant.name);
                        let hash = Hash::type_hash(&item);

                        self.install_type_info(ContextType {
                            item: item.clone(),
                            hash,
                            type_check: None,
                            type_info: TypeInfo::Variant(Arc::new(VariantRtti {
                                enum_hash: ty.hash,
                                hash,
                                item: item.clone(),
                            })),
                            type_parameters: Hash::EMPTY,
                        })?;

                        let constructor = if let Some(c) = &variant.constructor {
                            let signature = meta::Signature {
                                #[cfg(feature = "doc")]
                                is_async: false,
                                #[cfg(feature = "doc")]
                                args: Some(match fields {
                                    Fields::Named(names) => names.len(),
                                    Fields::Unnamed(args) => *args,
                                    Fields::Empty => 0,
                                }),
                                #[cfg(feature = "doc")]
                                return_type: Some(ty.hash),
                                #[cfg(feature = "doc")]
                                argument_types: Box::from([]),
                            };

                            self.insert_native_fn(hash, c)?;
                            Some(signature)
                        } else {
                            None
                        };

                        self.install_meta(ContextMeta {
                            hash,
                            item: Some(item),
                            kind: meta::Kind::Variant {
                                enum_hash: ty.hash,
                                index,
                                fields: match fields {
                                    Fields::Named(names) => {
                                        meta::Fields::Named(meta::FieldsNamed {
                                            fields: names
                                                .iter()
                                                .copied()
                                                .map(Box::<str>::from)
                                                .collect(),
                                        })
                                    }
                                    Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                                    Fields::Empty => meta::Fields::Empty,
                                },
                                constructor,
                            },
                            #[cfg(feature = "doc")]
                            docs: variant.docs.clone(),
                        })?;
                    }

                    meta::Kind::Enum { parameters }
                }
            }
        } else {
            meta::Kind::Type { parameters }
        };

        self.install_meta(ContextMeta {
            hash: ty.hash,
            item: Some(item),
            kind,
            #[cfg(feature = "doc")]
            docs: ty.docs.clone(),
        })?;

        Ok(())
    }

    fn install_type_info(&mut self, ty: ContextType) -> Result<(), ContextError> {
        let item_hash = Hash::type_hash(&ty.item).with_type_parameters(ty.type_parameters);

        if ty.hash != item_hash {
            return Err(ContextError::TypeHashMismatch {
                type_info: ty.type_info,
                item: ty.item,
                hash: ty.hash,
                item_hash,
            });
        }

        self.constants.insert(
            Hash::associated_function(ty.hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(ty.item.to_string()),
        );

        if let Some(old) = self.types.insert(ty.hash, ty) {
            return Err(ContextError::ConflictingType {
                item: old.item,
                type_info: old.type_info,
                hash: old.hash,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_function(
        &mut self,
        module: &Module,
        f: &ModuleFunction,
    ) -> Result<(), ContextError> {
        let item = module.item.join(&f.item);
        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.constants.insert(
            Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        let signature = meta::Signature {
            #[cfg(feature = "doc")]
            is_async: f.is_async,
            #[cfg(feature = "doc")]
            args: f.args,
            #[cfg(feature = "doc")]
            return_type: f.return_type.as_ref().map(|f| f.hash),
            #[cfg(feature = "doc")]
            argument_types: f
                .argument_types
                .iter()
                .map(|f| f.as_ref().map(|f| f.hash))
                .collect(),
        };

        self.insert_native_fn(hash, &f.handler)?;

        self.install_meta(ContextMeta {
            hash,
            item: Some(item),
            kind: meta::Kind::Function {
                is_test: false,
                is_bench: false,
                signature,
                parameters: Hash::EMPTY,
            },
            #[cfg(feature = "doc")]
            docs: f.docs.clone(),
        })?;

        Ok(())
    }

    /// Install a macro and check for duplicates.
    fn install_macro(&mut self, module: &Module, m: &ModuleMacro) -> Result<(), ContextError> {
        let item = module.item.join(&m.item);
        let hash = Hash::type_hash(&item);
        self.macros.insert(hash, m.handler.clone());

        self.install_meta(ContextMeta {
            hash,
            item: Some(item),
            kind: meta::Kind::Macro,
            #[cfg(feature = "doc")]
            docs: m.docs.clone(),
        })?;

        Ok(())
    }

    /// Install an attribute macro and check for duplicates.
    fn install_attribute_macro(
        &mut self,
        module: &Module,
        m: &ModuleAttributeMacro,
    ) -> Result<(), ContextError> {
        let item = module.item.join(&m.item);
        let hash = Hash::type_hash(&item);
        self.attribute_macros.insert(hash, m.handler.clone());

        self.install_meta(ContextMeta {
            hash,
            item: Some(item),
            kind: meta::Kind::AttributeMacro,
            #[cfg(feature = "doc")]
            docs: m.docs.clone(),
        })?;

        Ok(())
    }

    /// Install a constant and check for duplicates.
    fn install_constant(
        &mut self,
        module: &Module,
        m: &ModuleConstant,
    ) -> Result<(), ContextError> {
        let item = module.item.join(&m.item);
        let hash = Hash::type_hash(&item);
        self.constants.insert(hash, m.value.clone());

        self.install_meta(ContextMeta {
            hash,
            item: Some(item),
            kind: meta::Kind::Const,
            #[cfg(feature = "doc")]
            docs: m.docs.clone(),
        })?;

        Ok(())
    }

    fn install_associated(&mut self, assoc: &ModuleAssociated) -> Result<(), ContextError> {
        let Some(info) = self.types.get(&assoc.container.hash).cloned() else {
            return Err(ContextError::MissingContainer {
                container: assoc.container_type_info.clone(),
            });
        };

        // NB: `assoc.container.hash` already contains the type hash, so it
        // should not be mixed in again.
        let hash = assoc
            .name
            .kind
            .hash(assoc.container.hash)
            .with_function_parameters(assoc.name.function_parameters);

        let signature = meta::Signature {
            #[cfg(feature = "doc")]
            is_async: assoc.is_async,
            #[cfg(feature = "doc")]
            args: assoc.args,
            #[cfg(feature = "doc")]
            return_type: assoc.return_type.as_ref().map(|f| f.hash),
            #[cfg(feature = "doc")]
            argument_types: assoc
                .argument_types
                .iter()
                .map(|f| f.as_ref().map(|f| f.hash))
                .collect(),
        };

        self.insert_native_fn(hash, &assoc.handler)?;

        // If the associated function is a named instance function - register it
        // under the name of the item it corresponds to unless it's a field
        // function.
        //
        // The other alternatives are protocol functions (which are not free)
        // and plain hashes.
        let item = if let meta::AssociatedKind::Instance(name) = &assoc.name.kind {
            let item = info.item.extended(name.as_ref());

            let hash = Hash::type_hash(&item)
                .with_type_parameters(info.type_parameters)
                .with_function_parameters(assoc.name.function_parameters);

            self.constants.insert(
                Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
                ConstValue::String(item.to_string()),
            );

            self.insert_native_fn(hash, &assoc.handler)?;
            Some(item)
        } else {
            None
        };

        self.install_meta(ContextMeta {
            hash,
            item,
            kind: meta::Kind::AssociatedFunction {
                kind: assoc.name.kind.clone(),
                signature,
                parameters: Hash::EMPTY
                    .with_type_parameters(info.type_parameters)
                    .with_function_parameters(assoc.name.function_parameters),
                #[cfg(feature = "doc")]
                container: assoc.container.hash,
                #[cfg(feature = "doc")]
                parameter_types: assoc.name.parameter_types.clone(),
            },
            #[cfg(feature = "doc")]
            docs: assoc.docs.clone(),
        })?;

        Ok(())
    }

    /// Install unit type.
    fn install_unit_type(
        &mut self,
        module: &Module,
        unit_type: &UnitType,
    ) -> Result<(), ContextError> {
        let item = module.item.extended(&*unit_type.name);

        self.install_type_info(ContextType {
            item: item.clone(),
            hash: crate::runtime::UNIT_TYPE.hash,
            type_check: Some(TypeCheck::Unit),
            type_info: crate::runtime::UNIT_TYPE.type_info(),
            type_parameters: Hash::EMPTY,
        })?;

        let hash = <() as TypeOf>::type_hash();

        let signature = meta::Signature {
            #[cfg(feature = "doc")]
            is_async: false,
            #[cfg(feature = "doc")]
            args: Some(0),
            #[cfg(feature = "doc")]
            return_type: Some(hash),
            #[cfg(feature = "doc")]
            argument_types: Box::from([]),
        };

        let constructor = || ();
        let handler: Arc<FunctionHandler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.insert_native_fn(hash, &handler)?;

        self.install_meta(ContextMeta {
            hash,
            item: Some(item.clone()),
            kind: meta::Kind::Struct {
                fields: meta::Fields::Unnamed(0),
                constructor: Some(signature),
                parameters: Hash::EMPTY,
            },
            #[cfg(feature = "doc")]
            docs: unit_type.docs.clone(),
        })?;

        self.constants.insert(
            Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        Ok(())
    }

    /// Install generator state types.
    fn install_internal_enum(
        &mut self,
        module: &Module,
        internal_enum: &InternalEnum,
    ) -> Result<(), ContextError> {
        if !self.internal_enums.insert(internal_enum.static_type) {
            return Err(ContextError::InternalAlreadyPresent {
                name: internal_enum.name,
            });
        }

        let item = module.item.join(&internal_enum.base_type);

        let enum_hash = internal_enum.static_type.hash;

        self.install_meta(ContextMeta {
            hash: enum_hash,
            item: Some(item.clone()),
            kind: meta::Kind::Enum {
                parameters: Hash::EMPTY,
            },
            #[cfg(feature = "doc")]
            docs: internal_enum.docs.clone(),
        })?;

        self.install_type_info(ContextType {
            item: item.clone(),
            hash: enum_hash,
            type_check: None,
            type_info: internal_enum.static_type.type_info(),
            type_parameters: Hash::EMPTY,
        })?;

        for (index, variant) in internal_enum.variants.iter().enumerate() {
            let Some(fields) = &variant.fields else {
                continue;
            };

            let item = item.extended(variant.name);
            let hash = Hash::type_hash(&item);

            self.install_type_info(ContextType {
                item: item.clone(),
                hash,
                type_check: variant.type_check,
                type_info: internal_enum.static_type.type_info(),
                type_parameters: Hash::EMPTY,
            })?;

            let constructor = if let Some(constructor) = &variant.constructor {
                self.insert_native_fn(hash, constructor)?;

                Some(meta::Signature {
                    #[cfg(feature = "doc")]
                    is_async: false,
                    #[cfg(feature = "doc")]
                    args: Some(match fields {
                        Fields::Named(names) => names.len(),
                        Fields::Unnamed(args) => *args,
                        Fields::Empty => 0,
                    }),
                    #[cfg(feature = "doc")]
                    return_type: Some(enum_hash),
                    #[cfg(feature = "doc")]
                    argument_types: Box::from([]),
                })
            } else {
                None
            };

            self.install_meta(ContextMeta {
                hash,
                item: Some(item),
                kind: meta::Kind::Variant {
                    enum_hash,
                    index,
                    fields: match fields {
                        Fields::Named(fields) => meta::Fields::Named(meta::FieldsNamed {
                            fields: fields.iter().copied().map(Box::<str>::from).collect(),
                        }),
                        Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                        Fields::Empty => meta::Fields::Empty,
                    },
                    constructor,
                },
                #[cfg(feature = "doc")]
                docs: variant.docs.clone(),
            })?;
        }

        Ok(())
    }

    fn insert_native_fn(
        &mut self,
        hash: Hash,
        handler: &Arc<FunctionHandler>,
    ) -> Result<(), ContextError> {
        if self.functions.contains_key(&hash) {
            return Err(ContextError::ConflictingFunction { hash });
        }

        self.functions.insert(hash, handler.clone());
        Ok(())
    }

    /// Get a constant value.
    pub(crate) fn get_const_value(&self, hash: Hash) -> Option<&ConstValue> {
        self.constants.get(&hash)
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Context")
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send, Sync);
