use core::fmt;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeSet, Box, HashMap, HashSet, Vec};
use crate::compile::meta;
#[cfg(feature = "doc")]
use crate::compile::Docs;
#[cfg(feature = "emit")]
use crate::compile::MetaInfo;
use crate::compile::{ComponentRef, ContextError, IntoComponent, Item, ItemBuf, Names};
use crate::hash;
use crate::module::{
    Fields, Module, ModuleAssociated, ModuleAssociatedKind, ModuleItem, ModuleType,
    TypeSpecification,
};
use crate::runtime::{
    AttributeMacroHandler, ConstValue, FunctionHandler, MacroHandler, Protocol, RuntimeContext,
    StaticType, TypeCheck, TypeInfo, VariantRtti,
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
    /// Deprecation notice.
    #[cfg(feature = "doc")]
    pub(crate) deprecated: Option<Box<str>>,
    /// Documentation associated with a context meta.
    #[cfg(feature = "doc")]
    pub(crate) docs: Docs,
}

impl ContextMeta {
    #[cfg(feature = "emit")]
    pub(crate) fn info(&self) -> alloc::Result<MetaInfo> {
        MetaInfo::new(&self.kind, self.hash, self.item.as_deref())
    }
}

/// Information on a specific type.
#[derive(Debug, TryClone)]
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
    functions: hash::Map<Arc<FunctionHandler>>,
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
    constants: hash::Map<ConstValue>,
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

        this.install(crate::modules::num::module()?)?;
        this.install(crate::modules::any::module()?)?;
        this.install(crate::modules::bytes::module()?)?;
        this.install(crate::modules::char::module()?)?;
        this.install(crate::modules::hash::module()?)?;
        this.install(crate::modules::cmp::module()?)?;
        this.install(crate::modules::collections::module()?)?;
        this.install(crate::modules::f64::module()?)?;
        this.install(crate::modules::tuple::module()?)?;
        this.install(crate::modules::fmt::module()?)?;
        this.install(crate::modules::future::module()?)?;
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
    /// let runtime = Arc::new(context.runtime()?);
    /// let unit = Arc::new(Unit::default());
    ///
    /// let vm = Vm::new(runtime, unit);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn runtime(&self) -> alloc::Result<RuntimeContext> {
        Ok(RuntimeContext::new(
            self.functions.try_clone()?,
            self.constants.try_clone()?,
        ))
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
            if !self.unique.try_insert(id)? {
                return Ok(());
            }
        }

        if let Some(ComponentRef::Crate(name)) = module.item.first() {
            self.crates.try_insert(name.try_into()?)?;
        }

        self.install_module(module)?;

        for ty in &module.types {
            self.install_type(module, ty)?;
        }

        for item in &module.items {
            self.install_item(module, item)?;
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
    ) -> alloc::Result<impl Iterator<Item = ComponentRef<'a>> + 'a>
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
    pub(crate) fn contains_prefix(&self, item: &Item) -> alloc::Result<bool> {
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
            self.names.insert(item)?;

            self.item_to_hash
                .entry(item.try_clone()?)
                .or_try_default()?
                .try_insert(meta.hash)?;
        }

        #[cfg(feature = "doc")]
        if let Some(h) = meta.kind.associated_container() {
            let assoc = self.associated.entry(h).or_try_default()?;
            assoc.try_push(meta.hash)?;
        }

        let hash = meta.hash;
        let index = self.meta.len();
        self.meta.try_push(meta)?;
        self.hash_to_meta
            .entry(hash)
            .or_try_default()?
            .try_push(index)?;
        Ok(())
    }

    /// Install a module, ensuring that its meta is defined.
    fn install_module(&mut self, m: &Module) -> Result<(), ContextError> {
        self.names.insert(&m.item)?;

        let mut current = Some((m.item.as_ref(), Some(&m.common)));

        #[allow(unused)]
        while let Some((item, common)) = current.take() {
            self.install_meta(ContextMeta {
                hash: Hash::type_hash(item),
                item: Some(item.try_to_owned()?),
                kind: meta::Kind::Module,
                #[cfg(feature = "doc")]
                deprecated: common
                    .map(|c| c.deprecated.as_ref().try_cloned())
                    .transpose()?
                    .flatten(),
                #[cfg(feature = "doc")]
                docs: common
                    .map(|c| c.docs.try_clone())
                    .transpose()?
                    .unwrap_or_default(),
            })?;

            current = item.parent().map(|item| (item, None));
        }

        Ok(())
    }

    /// Install a single type.
    fn install_type(&mut self, module: &Module, ty: &ModuleType) -> Result<(), ContextError> {
        let item = module.item.join(&ty.item)?;

        self.install_type_info(ContextType {
            item: item.try_clone()?,
            hash: ty.hash,
            type_check: None,
            type_info: ty.type_info.try_clone()?,
            type_parameters: ty.type_parameters,
        })?;

        let parameters = Hash::EMPTY.with_type_parameters(ty.type_parameters);

        let kind = if let Some(spec) = &ty.spec {
            match spec {
                TypeSpecification::Struct(fields) => {
                    let constructor = match &ty.constructor {
                        Some(c) => {
                            let hash = Hash::type_hash(&item);

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
                                argument_types: Box::default(),
                            };

                            self.insert_native_fn(hash, c)?;
                            Some(signature)
                        }
                        None => None,
                    };

                    meta::Kind::Struct {
                        fields: match fields {
                            Fields::Named(fields) => meta::Fields::Named(meta::FieldsNamed {
                                fields: fields
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .map(|(position, name)| {
                                        Ok((
                                            Box::<str>::try_from(name)?,
                                            meta::FieldMeta { position },
                                        ))
                                    })
                                    .try_collect::<alloc::Result<_>>()??,
                            }),
                            Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                            Fields::Empty => meta::Fields::Empty,
                        },
                        constructor,
                        parameters,
                    }
                }
                TypeSpecification::Enum(en) => {
                    for (index, variant) in en.variants.iter().enumerate() {
                        let Some(fields) = &variant.fields else {
                            continue;
                        };

                        let item = item.extended(variant.name)?;
                        let hash = Hash::type_hash(&item);

                        self.install_type_info(ContextType {
                            item: item.try_clone()?,
                            hash,
                            type_check: None,
                            type_info: TypeInfo::Variant(Arc::new(VariantRtti {
                                enum_hash: ty.hash,
                                hash,
                                item: item.try_clone()?,
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
                                argument_types: Box::default(),
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
                                                .enumerate()
                                                .map(|(position, name)| {
                                                    Ok((
                                                        Box::<str>::try_from(name)?,
                                                        meta::FieldMeta { position },
                                                    ))
                                                })
                                                .try_collect::<alloc::Result<_>>()??,
                                        })
                                    }
                                    Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                                    Fields::Empty => meta::Fields::Empty,
                                },
                                constructor,
                            },
                            #[cfg(feature = "doc")]
                            deprecated: variant.deprecated.try_clone()?,
                            #[cfg(feature = "doc")]
                            docs: variant.docs.try_clone()?,
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
            deprecated: ty.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: ty.common.docs.try_clone()?,
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

        self.constants.try_insert(
            Hash::associated_function(ty.hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(ty.item.try_to_string()?),
        )?;

        if let Some(old) = self.types.try_insert(ty.hash, ty)? {
            return Err(ContextError::ConflictingType {
                item: old.item,
                type_info: old.type_info,
                hash: old.hash,
            });
        }

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_item(
        &mut self,
        module: &Module,
        module_item: &ModuleItem,
    ) -> Result<(), ContextError> {
        let item = module.item.join(&module_item.item)?;
        self.names.insert(&item)?;

        let hash = Hash::type_hash(&item);

        let kind = match &module_item.kind {
            rune::module::ModuleItemKind::Constant(value) => {
                self.constants.try_insert(hash, value.try_clone()?)?;
                meta::Kind::Const
            }
            rune::module::ModuleItemKind::Function(f) => {
                self.constants.try_insert(
                    Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(item.try_to_string()?),
                )?;

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
                        .try_collect()?,
                };

                self.insert_native_fn(hash, &f.handler)?;

                meta::Kind::Function {
                    associated: None,
                    signature,
                    is_test: false,
                    is_bench: false,
                    parameters: Hash::EMPTY,
                    #[cfg(feature = "doc")]
                    container: None,
                    #[cfg(feature = "doc")]
                    parameter_types: Vec::new(),
                }
            }
            rune::module::ModuleItemKind::Macro(m) => {
                self.macros.try_insert(hash, m.handler.clone())?;
                meta::Kind::Macro
            }
            rune::module::ModuleItemKind::AttributeMacro(m) => {
                self.attribute_macros.try_insert(hash, m.handler.clone())?;
                meta::Kind::AttributeMacro
            }
            rune::module::ModuleItemKind::InternalEnum(internal_enum) => {
                if !self.internal_enums.try_insert(internal_enum.static_type)? {
                    return Err(ContextError::InternalAlreadyPresent {
                        name: internal_enum.name,
                    });
                }

                // Sanity check that the registered item is in the right location.
                if internal_enum.static_type.hash != hash {
                    return Err(ContextError::TypeHashMismatch {
                        type_info: internal_enum.static_type.type_info(),
                        item,
                        hash: internal_enum.static_type.hash,
                        item_hash: hash,
                    });
                }

                self.install_type_info(ContextType {
                    item: item.try_clone()?,
                    hash,
                    type_check: None,
                    type_info: internal_enum.static_type.type_info(),
                    type_parameters: Hash::EMPTY,
                })?;

                for (index, variant) in internal_enum.variants.iter().enumerate() {
                    let Some(fields) = &variant.fields else {
                        continue;
                    };

                    let variant_item = item.extended(variant.name)?;
                    let variant_hash = Hash::type_hash(&variant_item);

                    self.install_type_info(ContextType {
                        item: variant_item.try_clone()?,
                        hash: variant_hash,
                        type_check: variant.type_check,
                        type_info: internal_enum.static_type.type_info(),
                        type_parameters: Hash::EMPTY,
                    })?;

                    let constructor = if let Some(constructor) = &variant.constructor {
                        self.insert_native_fn(variant_hash, constructor)?;

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
                            return_type: Some(hash),
                            #[cfg(feature = "doc")]
                            argument_types: Box::default(),
                        })
                    } else {
                        None
                    };

                    self.install_meta(ContextMeta {
                        hash: variant_hash,
                        item: Some(variant_item),
                        kind: meta::Kind::Variant {
                            enum_hash: hash,
                            index,
                            fields: match fields {
                                Fields::Named(fields) => meta::Fields::Named(meta::FieldsNamed {
                                    fields: fields
                                        .iter()
                                        .copied()
                                        .enumerate()
                                        .map(|(position, name)| {
                                            Ok((
                                                Box::<str>::try_from(name)?,
                                                meta::FieldMeta { position },
                                            ))
                                        })
                                        .try_collect::<alloc::Result<_>>()??,
                                }),
                                Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                                Fields::Empty => meta::Fields::Empty,
                            },
                            constructor,
                        },
                        #[cfg(feature = "doc")]
                        deprecated: variant.deprecated.try_clone()?,
                        #[cfg(feature = "doc")]
                        docs: variant.docs.try_clone()?,
                    })?;
                }

                meta::Kind::Enum {
                    parameters: Hash::EMPTY,
                }
            }
        };

        self.install_meta(ContextMeta {
            hash,
            item: Some(item),
            kind,
            #[cfg(feature = "doc")]
            deprecated: module_item.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: module_item.common.docs.try_clone()?,
        })?;

        Ok(())
    }

    fn install_associated(&mut self, assoc: &ModuleAssociated) -> Result<(), ContextError> {
        let Some(info) = self.types.get(&assoc.container.hash).try_cloned()? else {
            return Err(ContextError::MissingContainer {
                container: assoc.container_type_info.try_clone()?,
            });
        };

        // NB: `assoc.container.hash` already contains the type hash, so it
        // should not be mixed in again.
        let hash = assoc
            .name
            .kind
            .hash(assoc.container.hash)
            .with_function_parameters(assoc.name.function_parameters);

        // If the associated function is a named instance function - register it
        // under the name of the item it corresponds to unless it's a field
        // function.
        //
        // The other alternatives are protocol functions (which are not free)
        // and plain hashes.
        let item = if let meta::AssociatedKind::Instance(name) = &assoc.name.kind {
            let item = info.item.extended(name.as_ref())?;

            let hash = Hash::type_hash(&item)
                .with_type_parameters(info.type_parameters)
                .with_function_parameters(assoc.name.function_parameters);

            Some((hash, item))
        } else {
            None
        };

        let kind = match &assoc.kind {
            ModuleAssociatedKind::Constant(value) => {
                if let Some((hash, ..)) = &item {
                    self.constants.try_insert(*hash, value.try_clone()?)?;
                }

                self.constants.try_insert(hash, value.try_clone()?)?;
                meta::Kind::Const
            }
            ModuleAssociatedKind::Function(f) => {
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
                        .try_collect()?,
                };

                if let Some((hash, item)) = &item {
                    self.constants.try_insert(
                        Hash::associated_function(*hash, Protocol::INTO_TYPE_NAME),
                        ConstValue::String(item.try_to_string()?),
                    )?;

                    self.insert_native_fn(*hash, &f.handler)?;
                }

                self.insert_native_fn(hash, &f.handler)?;

                meta::Kind::Function {
                    associated: Some(assoc.name.kind.try_clone()?),
                    signature,
                    is_test: false,
                    is_bench: false,
                    parameters: Hash::EMPTY
                        .with_type_parameters(info.type_parameters)
                        .with_function_parameters(assoc.name.function_parameters),
                    #[cfg(feature = "doc")]
                    container: Some(assoc.container.hash),
                    #[cfg(feature = "doc")]
                    parameter_types: assoc.name.parameter_types.try_clone()?,
                }
            }
        };

        self.install_meta(ContextMeta {
            hash,
            item: item.map(|(_, item)| item),
            kind,
            #[cfg(feature = "doc")]
            deprecated: assoc.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: assoc.common.docs.try_clone()?,
        })?;

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

        self.functions.try_insert(hash, handler.clone())?;
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
