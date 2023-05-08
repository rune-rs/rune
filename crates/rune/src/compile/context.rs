use core::fmt;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::collections::{HashMap, HashSet};
use crate::compile::meta;
use crate::compile::{
    ComponentRef, ContextError, Docs, IntoComponent, Item, ItemBuf, MetaInfo, Names,
};
use crate::module::{
    AssociatedKey, AssociatedKind, Function, InternalEnum, Module, ModuleAssociated,
    ModuleFunction, ModuleMacro, Type, TypeSpecification, UnitType, VariantKind,
};
use crate::runtime::{
    ConstValue, FunctionHandler, MacroHandler, Protocol, RuntimeContext, StaticType, TypeCheck,
    TypeInfo, TypeOf, VariantRtti,
};
use crate::Hash;

/// Context metadata.
#[non_exhaustive]
pub(crate) struct ContextMeta {
    /// Type hash for the given meta item.
    pub(crate) hash: Hash,
    /// The container this item belongs to.
    pub(crate) associated_container: Option<Hash>,
    /// The item of the returned compile meta.
    pub(crate) item: ItemBuf,
    /// The kind of the compile meta.
    pub(crate) kind: meta::Kind,
    /// Documentation associated with a context meta.
    #[cfg_attr(not(feature = "doc"), allow(unused))]
    pub(crate) docs: Docs,
}

impl ContextMeta {
    pub(crate) fn new(
        hash: Hash,
        associated_container: Option<Hash>,
        item: ItemBuf,
        kind: meta::Kind,
        docs: Docs,
    ) -> Self {
        Self {
            hash,
            associated_container,
            item,
            kind,
            docs,
        }
    }

    pub(crate) fn info(&self) -> MetaInfo {
        MetaInfo::new(&self.kind, &self.item)
    }
}

/// Information on a specific type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct PrivTypeInfo {
    /// The type check used for the current type.
    type_check: Option<TypeCheck>,
    /// Complete detailed information on the hash.
    type_info: TypeInfo,
    /// Item of the type.
    item: ItemBuf,
    /// The hash of the type.
    type_hash: Hash,
}

impl fmt::Display for PrivTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.item, self.type_info)?;
        Ok(())
    }
}

pub(crate) enum ContextAssociated {
    /// Associated self-receiver.
    Associated(ModuleAssociated),
    /// A simple module function.
    Function(Hash),
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
    /// Item metadata in the context.
    meta: HashMap<Hash, Vec<ContextMeta>>,
    /// Store item to hash mapping.
    item_to_hash: HashMap<ItemBuf, Hash>,
    /// Information on functions.
    functions_info: HashMap<Hash, meta::Signature>,
    /// Registered native function handlers.
    functions: HashMap<Hash, Arc<FunctionHandler>>,
    /// Information on associated types.
    #[cfg(feature = "doc")]
    associated: HashMap<Hash, Vec<ContextAssociated>>,
    /// Registered native macro handlers.
    macros: HashMap<Hash, Arc<MacroHandler>>,
    /// Registered types.
    types: HashMap<Hash, PrivTypeInfo>,
    /// Reverse lookup for types, which maps the item type hash to the internal
    /// type hash which is usually based on a type id.
    types_rev: HashMap<Hash, Hash>,
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
        this.install(crate::modules::float::module()?)?;
        this.install(crate::modules::fmt::module()?)?;
        this.install(crate::modules::future::module()?)?;
        this.install(crate::modules::generator::module()?)?;
        this.install(crate::modules::int::module()?)?;
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

        for (type_hash, ty) in &module.types {
            self.install_type(module, *type_hash, ty, Docs::default())?;
        }

        for (name, f) in &module.functions {
            self.install_function(module, name, f)?;
        }

        for (name, m) in &module.macros {
            self.install_macro(module, name, m)?;
        }

        for (name, m) in &module.constants {
            self.install_constant(module, name, m, Docs::default())?;
        }

        if let Some(unit_type) = &module.unit_type {
            self.install_unit_type(module, unit_type, Docs::default())?;
        }

        for internal_enum in &module.internal_enums {
            self.install_internal_enum(module, internal_enum, Docs::default())?;
        }

        for (key, assoc) in &module.associated {
            self.install_associated(key, assoc)?;
        }

        Ok(())
    }

    /// Iterate over all available functions in the [Context].
    #[cfg(feature = "cli")]
    pub(crate) fn iter_functions(&self) -> impl Iterator<Item = (Hash, &meta::Signature)> {
        use core::iter;

        let mut it = self.functions_info.iter();

        iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
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
    pub(crate) fn lookup_meta(&self, item: &Item) -> &[ContextMeta] {
        let Some(hash) = self.item_to_hash.get(item) else {
            return &[];
        };

        let Some(meta) = self.meta.get(hash) else {
            return &[];
        };

        meta
    }

    /// Lookup meta by its hash.
    #[cfg(feature = "doc")]
    pub(crate) fn lookup_meta_by_hash(&self, hash: Hash) -> &[ContextMeta] {
        self.meta.get(&hash).map(Vec::as_slice).unwrap_or_default()
    }

    /// Look up signature of function.
    #[cfg(feature = "doc")]
    pub(crate) fn lookup_signature(&self, hash: Hash) -> Option<&meta::Signature> {
        self.functions_info.get(&hash)
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
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = &ContextAssociated> + '_ {
        self.associated
            .get(&hash)
            .into_iter()
            .flat_map(|items| items.iter())
    }

    /// Lookup the given macro handler.
    pub(crate) fn lookup_macro(&self, hash: Hash) -> Option<&Arc<MacroHandler>> {
        self.macros.get(&hash)
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
        self.names.insert(&meta.item);

        if let Some(existing) = self.item_to_hash.insert(meta.item.clone(), meta.hash) {
            if meta.hash != existing {
                return Err(ContextError::ConflictingMetaHash {
                    item: meta.item.clone(),
                    hash: meta.hash,
                    existing,
                });
            }
        }

        self.meta.entry(meta.hash).or_default().push(meta);
        Ok(())
    }

    /// Install a module, ensuring that its meta is defined.
    fn install_module(&mut self, m: &Module) -> Result<(), ContextError> {
        self.names.insert(&m.item);

        let mut current = Some((m.item.as_ref(), Some(&m.docs)));

        while let Some((item, docs)) = current.take() {
            let hash = Hash::type_hash(item);

            if let Some(existing) = self.item_to_hash.insert(item.to_owned(), hash) {
                if hash != existing {
                    return Err(ContextError::ConflictingMetaHash {
                        item: item.to_owned(),
                        hash,
                        existing,
                    });
                }
            }

            self.meta.entry(hash).or_default().push(ContextMeta {
                hash,
                associated_container: None,
                item: item.to_owned(),
                kind: meta::Kind::Module,
                docs: docs.cloned().unwrap_or_default(),
            });

            current = item.parent().map(|item| (item, None));
        }

        Ok(())
    }

    /// Install a single type.
    fn install_type(
        &mut self,
        module: &Module,
        type_hash: Hash,
        ty: &Type,
        docs: Docs,
    ) -> Result<(), ContextError> {
        let item = module.item.extended(&*ty.name);
        let hash = Hash::type_hash(&item);

        self.install_type_info(
            hash,
            PrivTypeInfo {
                type_check: None,
                item: item.clone(),
                type_hash,
                type_info: ty.type_info.clone(),
            },
        )?;

        let kind = if let Some(spec) = &ty.spec {
            match spec {
                TypeSpecification::Struct(st) => meta::Kind::Struct {
                    fields: meta::Fields::Struct(meta::Struct {
                        fields: st.fields.clone(),
                    }),
                },
                TypeSpecification::Enum(en) => {
                    let enum_item = &item;
                    let enum_hash = type_hash;

                    for (index, (name, variant)) in en.variants.iter().enumerate() {
                        let item = enum_item.extended(name);
                        let hash = Hash::type_hash(&item);
                        let constructor = variant.constructor.as_ref();

                        let (fields, args) = match &variant.kind {
                            VariantKind::Tuple(t) => (
                                meta::Fields::Tuple(meta::Tuple { args: t.args, hash }),
                                Some(t.args),
                            ),
                            VariantKind::Struct(st) => (
                                meta::Fields::Struct(meta::Struct {
                                    fields: st.fields.clone(),
                                }),
                                None,
                            ),
                            VariantKind::Unit => (meta::Fields::Unit, Some(0)),
                        };

                        self.install_type_info(
                            hash,
                            PrivTypeInfo {
                                type_check: None,
                                item: item.clone(),
                                type_hash: hash,
                                type_info: TypeInfo::Variant(Arc::new(VariantRtti {
                                    enum_hash,
                                    hash,
                                    item: item.clone(),
                                })),
                            },
                        )?;

                        if let (Some(c), Some(args)) = (constructor, args) {
                            let signature = meta::Signature {
                                item: item.clone(),
                                is_async: false,
                                args: Some(args),
                                return_type: Some(enum_hash),
                                argument_types: Box::from([]),
                                kind: meta::SignatureKind::Function,
                            };

                            if let Some(old) = self.functions_info.insert(hash, signature) {
                                return Err(ContextError::ConflictingFunction {
                                    signature: Box::new(old),
                                    hash,
                                });
                            }

                            self.functions.insert(hash, c.clone());
                        }

                        let kind = meta::Kind::Variant {
                            enum_hash,
                            index,
                            fields,
                        };

                        self.install_meta(ContextMeta::new(
                            hash,
                            Some(enum_hash),
                            item,
                            kind,
                            Docs::default(),
                        ))?;
                    }

                    meta::Kind::Enum
                }
            }
        } else {
            meta::Kind::Type
        };

        self.install_meta(ContextMeta::new(type_hash, None, item, kind, docs))?;
        Ok(())
    }

    fn install_type_info(&mut self, hash: Hash, info: PrivTypeInfo) -> Result<(), ContextError> {
        // reverse lookup for types.
        if let Some(existing) = self.types_rev.insert(info.type_hash, hash) {
            return Err(ContextError::ConflictingTypeHash { hash, existing });
        }

        self.constants.insert(
            Hash::instance_function(info.type_hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(info.item.to_string()),
        );

        if let Some(old) = self.types.insert(hash, info) {
            return Err(ContextError::ConflictingType {
                item: old.item,
                type_info: old.type_info,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_function(
        &mut self,
        module: &Module,
        item: &Item,
        f: &ModuleFunction,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);
        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        let signature = meta::Signature {
            item: item.clone(),
            is_async: f.is_async,
            args: f.args,
            return_type: f.return_type.as_ref().map(|f| f.hash),
            argument_types: f
                .argument_types
                .iter()
                .map(|f| f.as_ref().map(|f| f.hash))
                .collect(),
            kind: meta::SignatureKind::Function,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: Box::new(old),
                hash,
            });
        }

        self.functions.insert(hash, f.handler.clone());

        self.install_meta(ContextMeta::new(
            hash,
            None,
            item,
            meta::Kind::Function {
                is_async: f.is_async,
                args: f.args,
                is_test: false,
                is_bench: false,
                instance_function: false,
            },
            f.docs.clone(),
        ))?;

        #[cfg(feature = "doc")]
        if let Some(container_hash) = f.associated_container {
            self.associated
                .entry(container_hash)
                .or_default()
                .push(ContextAssociated::Function(hash));
        }

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
        let hash = Hash::type_hash(&item);
        self.macros.insert(hash, m.handler.clone());

        self.install_meta(ContextMeta::new(
            hash,
            None,
            item,
            meta::Kind::Macro,
            m.docs.clone(),
        ))?;

        Ok(())
    }

    /// Install a constant and check for duplicates.
    fn install_constant(
        &mut self,
        module: &Module,
        item: &Item,
        v: &ConstValue,
        docs: Docs,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);
        let hash = Hash::type_hash(&item);
        self.constants.insert(hash, v.clone());

        self.install_meta(ContextMeta::new(
            hash,
            None,
            item,
            meta::Kind::Const {
                const_value: v.clone(),
            },
            docs,
        ))?;

        Ok(())
    }

    fn install_associated(
        &mut self,
        key: &AssociatedKey,
        assoc: &ModuleAssociated,
    ) -> Result<(), ContextError> {
        let info = match self
            .types_rev
            .get(&key.type_hash)
            .and_then(|hash| self.types.get(hash))
        {
            Some(info) => info,
            None => {
                return Err(ContextError::MissingInstance {
                    instance_type: assoc.type_info.clone(),
                });
            }
        };

        let hash = assoc
            .name
            .kind
            .hash(key.type_hash)
            .with_parameters(key.parameters);

        let signature = meta::Signature {
            item: info.item.clone(),
            is_async: assoc.is_async,
            args: assoc.args,
            return_type: assoc.return_type.as_ref().map(|f| f.hash),
            argument_types: assoc
                .argument_types
                .iter()
                .map(|f| f.as_ref().map(|f| f.hash))
                .collect(),
            kind: meta::SignatureKind::Instance {
                name: assoc.name.kind.clone(),
                self_type_info: info.type_info.clone(),
            },
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: Box::new(old),
                hash,
            });
        }

        self.functions.insert(hash, assoc.handler.clone());

        #[cfg(feature = "doc")]
        self.associated
            .entry(key.type_hash)
            .or_default()
            .push(ContextAssociated::Associated(assoc.clone()));

        // If the associated function is a named instance function - register it
        // under the name of the item it corresponds to unless it's a field
        // function.
        //
        // The other alternatives are protocol functions (which are not free)
        // and plain hashes.
        if let AssociatedKind::Instance(name) = &assoc.name.kind {
            let item = info.item.extended(name);
            let type_hash = Hash::type_hash(&item);
            let hash = type_hash.with_parameters(key.parameters);

            self.constants.insert(
                Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
                ConstValue::String(item.to_string()),
            );

            let signature = meta::Signature {
                item: item.clone(),
                is_async: assoc.is_async,
                args: assoc.args,
                return_type: assoc.return_type.as_ref().map(|f| f.hash),
                argument_types: assoc
                    .argument_types
                    .iter()
                    .map(|f| f.as_ref().map(|f| f.hash))
                    .collect(),
                kind: meta::SignatureKind::Function,
            };

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: Box::new(old),
                    hash,
                });
            }

            self.functions.insert(hash, assoc.handler.clone());

            // TODO: remove check since we now have multi meta?
            if !self.item_to_hash.contains_key(&item) {
                self.install_meta(ContextMeta::new(
                    type_hash,
                    Some(key.type_hash),
                    item,
                    meta::Kind::Function {
                        is_async: assoc.is_async,
                        args: assoc.args,
                        is_test: false,
                        is_bench: false,
                        instance_function: true,
                    },
                    assoc.docs.clone(),
                ))?;
            }
        }

        Ok(())
    }

    /// Install unit type.
    fn install_unit_type(
        &mut self,
        module: &Module,
        unit_type: &UnitType,
        docs: Docs,
    ) -> Result<(), ContextError> {
        let item = module.item.extended(&*unit_type.name);
        let hash = Hash::type_hash(&item);
        self.add_internal_tuple(None, item.clone(), 0, || (), docs)?;

        self.install_type_info(
            hash,
            PrivTypeInfo {
                type_check: Some(TypeCheck::Unit),
                item,
                type_hash: crate::runtime::UNIT_TYPE.hash,
                type_info: TypeInfo::StaticType(crate::runtime::UNIT_TYPE),
            },
        )?;

        Ok(())
    }

    /// Install generator state types.
    fn install_internal_enum(
        &mut self,
        module: &Module,
        internal_enum: &InternalEnum,
        docs: Docs,
    ) -> Result<(), ContextError> {
        if !self.internal_enums.insert(internal_enum.static_type) {
            return Err(ContextError::InternalAlreadyPresent {
                name: internal_enum.name,
            });
        }

        let enum_item = module.item.join(&internal_enum.base_type);
        let enum_hash = Hash::type_hash(&enum_item);

        self.install_meta(ContextMeta::new(
            internal_enum.static_type.hash,
            None,
            enum_item.clone(),
            meta::Kind::Enum,
            docs,
        ))?;

        self.install_type_info(
            enum_hash,
            PrivTypeInfo {
                type_check: None,
                item: enum_item.clone(),
                type_hash: internal_enum.static_type.hash,
                type_info: TypeInfo::StaticType(internal_enum.static_type),
            },
        )?;

        for (index, variant) in internal_enum.variants.iter().enumerate() {
            let item = enum_item.extended(variant.name);
            let hash = Hash::type_hash(&item);

            self.install_type_info(
                hash,
                PrivTypeInfo {
                    type_check: Some(variant.type_check),
                    item: item.clone(),
                    type_hash: hash,
                    type_info: TypeInfo::StaticType(internal_enum.static_type),
                },
            )?;

            self.install_meta(ContextMeta::new(
                hash,
                Some(internal_enum.static_type.hash),
                item.clone(),
                meta::Kind::Variant {
                    enum_hash,
                    index,
                    fields: meta::Fields::Tuple(meta::Tuple {
                        args: variant.args,
                        hash,
                    }),
                },
                Docs::default(),
            ))?;

            let signature = meta::Signature {
                item,
                is_async: false,
                args: Some(variant.args),
                return_type: Some(enum_hash),
                argument_types: Box::from([]),
                kind: meta::SignatureKind::Function,
            };

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: Box::new(old),
                    hash,
                });
            }

            self.functions.insert(hash, variant.constructor.clone());
        }

        Ok(())
    }

    /// Add a piece of internal tuple meta.
    fn add_internal_tuple<C, A>(
        &mut self,
        enum_item: Option<(Hash, usize)>,
        item: ItemBuf,
        args: usize,
        constructor: C,
        docs: Docs,
    ) -> Result<(), ContextError>
    where
        C: Function<A>,
        C::Return: TypeOf,
    {
        let type_hash = <C::Return as TypeOf>::type_hash();
        let hash = Hash::type_hash(&item);

        let tuple = meta::Tuple { args, hash };

        let priv_meta = match enum_item {
            Some((enum_hash, index)) => ContextMeta::new(
                type_hash,
                Some(enum_hash),
                item.clone(),
                meta::Kind::Variant {
                    enum_hash,
                    index,
                    fields: meta::Fields::Tuple(tuple),
                },
                docs,
            ),
            None => ContextMeta::new(
                type_hash,
                None,
                item.clone(),
                meta::Kind::Struct {
                    fields: meta::Fields::Tuple(tuple),
                },
                docs,
            ),
        };

        self.install_meta(priv_meta)?;

        let constructor: Arc<FunctionHandler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.constants.insert(
            Hash::instance_function(type_hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        let signature = meta::Signature {
            item,
            is_async: false,
            args: Some(args),
            return_type: Some(type_hash),
            argument_types: Box::from([]),
            kind: meta::SignatureKind::Function,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: Box::new(old),
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
