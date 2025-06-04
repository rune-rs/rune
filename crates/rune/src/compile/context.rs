use core::fmt;

use rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeSet, Box, HashMap, HashSet, String, Vec};
#[cfg(feature = "emit")]
use crate::compile::MetaInfo;
use crate::compile::{self, ContextError, Names};
use crate::compile::{meta, Docs};
use crate::function::{Function, Plain};
use crate::function_meta::{AssociatedName, ToInstance};
use crate::hash;
use crate::item::{ComponentRef, IntoComponent};
use crate::macros::{MacroContext, TokenStream};
use crate::module::{
    DocFunction, Fields, Module, ModuleAssociated, ModuleAssociatedKind, ModuleFunction,
    ModuleItem, ModuleItemCommon, ModuleReexport, ModuleTrait, ModuleTraitImpl, ModuleType,
    TypeSpecification,
};
use crate::runtime::{
    AnyTypeInfo, ConstConstruct, ConstContext, ConstValue, FunctionHandler, InstAddress, Memory,
    Output, Protocol, Rtti, RttiKind, RuntimeContext, TypeCheck, TypeInfo, VmResult,
};
use crate::{Hash, Item, ItemBuf};

/// A (type erased) macro handler.
pub(crate) type MacroHandler =
    dyn Fn(&mut MacroContext, &TokenStream) -> compile::Result<TokenStream> + Send + Sync;

/// Invoked when types implement a trait.
pub(crate) type TraitHandler =
    dyn Fn(&mut TraitContext<'_>) -> Result<(), ContextError> + Send + Sync;

/// A (type erased) attribute macro handler.
pub(crate) type AttributeMacroHandler = dyn Fn(&mut MacroContext, &TokenStream, &TokenStream) -> compile::Result<TokenStream>
    + Send
    + Sync;

/// Type used to install traits.
pub struct TraitContext<'a> {
    /// The context the trait function are being installed into.
    cx: &'a mut Context,
    /// The item being installed.
    item: &'a Item,
    /// The hash of the item being installed.
    hash: Hash,
    /// Type info of the type being installed.
    type_info: &'a TypeInfo,
    /// The trait being implemented for.
    trait_item: &'a Item,
    /// Hash of the trait being impleemnted.
    trait_hash: Hash,
}

impl TraitContext<'_> {
    /// Return the item the trait is being installed for.
    pub fn item(&self) -> &Item {
        self.item
    }

    /// Return the hash the trait is being installed for.
    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// Find the given protocol function for the current type.
    ///
    /// This requires that the function is defined.
    pub fn find(
        &mut self,
        protocol: &'static Protocol,
    ) -> Result<Arc<FunctionHandler>, ContextError> {
        let name = protocol.to_instance()?;

        let hash = name
            .kind
            .hash(self.hash)
            .with_function_parameters(name.function_parameters);

        let Some(handler) = self.cx.functions.get(&hash) else {
            return Err(ContextError::MissingTraitFunction {
                name: name.kind.try_to_string()?,
                item: self.item.try_to_owned()?,
                hash,
                trait_item: self.trait_item.try_to_owned()?,
                trait_hash: self.trait_hash,
            });
        };

        let handler = handler.clone();

        if let Some(method) = protocol.method {
            self.function_handler(method, &handler)?;
        }

        Ok(handler)
    }

    /// Try to find the given associated function.
    ///
    /// This does not require that the function is defined.
    pub fn try_find(
        &self,
        name: impl ToInstance,
    ) -> Result<Option<Arc<FunctionHandler>>, ContextError> {
        let name = name.to_instance()?;

        let hash = name
            .kind
            .hash(self.hash)
            .with_function_parameters(name.function_parameters);

        Ok(self.cx.functions.get(&hash).cloned())
    }

    /// Find or define a protocol function.
    pub fn find_or_define<A, F>(
        &mut self,
        protocol: &'static Protocol,
        function: F,
    ) -> Result<Arc<FunctionHandler>, ContextError>
    where
        F: Function<A, Plain>,
    {
        let function = if let Some(function) = self.try_find(protocol)? {
            function
        } else {
            self.function(protocol, function)?
        };

        if let Some(method) = protocol.method {
            self.function_handler(method, &function)?;
        }

        Ok(function)
    }

    /// Define a new associated function for the current type.
    pub fn function<F, A>(
        &mut self,
        name: impl ToInstance,
        handler: F,
    ) -> Result<Arc<FunctionHandler>, ContextError>
    where
        F: Function<A, Plain>,
    {
        let handler: Arc<FunctionHandler> =
            Arc::new(move |memory, addr, len, out| handler.call(memory, addr, len, out));
        self.function_handler(name, &handler)?;
        Ok(handler)
    }

    /// Define a new associated raw function for the current type.
    pub fn raw_function<F>(
        &mut self,
        name: impl ToInstance,
        handler: F,
    ) -> Result<Arc<FunctionHandler>, ContextError>
    where
        F: 'static + Fn(&mut dyn Memory, InstAddress, usize, Output) -> VmResult<()> + Send + Sync,
    {
        let handler: Arc<FunctionHandler> =
            Arc::new(move |memory, addr, len, out| handler(memory, addr, len, out).into_result());
        self.function_handler(name, &handler)?;
        Ok(handler)
    }

    /// Define a new associated function for the current type using a raw
    /// handler.
    fn function_handler(
        &mut self,
        name: impl ToInstance,
        handler: &Arc<FunctionHandler>,
    ) -> Result<(), ContextError> {
        let name = name.to_instance()?;
        self.function_inner(name, handler)
    }

    fn function_inner(
        &mut self,
        name: AssociatedName,
        handler: &Arc<FunctionHandler>,
    ) -> Result<(), ContextError> {
        let function = ModuleFunction {
            handler: handler.clone(),
            trait_hash: Some(self.trait_hash),
            doc: DocFunction {
                #[cfg(feature = "doc")]
                is_async: false,
                #[cfg(feature = "doc")]
                args: None,
                #[cfg(feature = "doc")]
                argument_types: Box::default(),
                #[cfg(feature = "doc")]
                return_type: meta::DocType::empty(),
            },
        };

        let assoc = ModuleAssociated {
            container: self.hash,
            container_type_info: self.type_info.try_clone()?,
            name,
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
            kind: ModuleAssociatedKind::Function(function),
        };

        self.cx.install_associated(&assoc)?;
        Ok(())
    }
}

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
    hash_to_meta: hash::Map<Vec<usize>>,
    /// Store item to hash mapping.
    item_to_hash: HashMap<ItemBuf, BTreeSet<Hash>>,
    /// Registered native function handlers.
    functions: hash::Map<Arc<FunctionHandler>>,
    /// Registered deprecation mesages for native functions.
    deprecations: hash::Map<String>,
    /// Information on associated types.
    #[cfg(feature = "doc")]
    associated: hash::Map<Vec<Hash>>,
    /// Traits implemented by the given hash.
    #[cfg(feature = "doc")]
    implemented_traits: hash::Map<Vec<Hash>>,
    /// Registered native macro handlers.
    macros: hash::Map<Arc<MacroHandler>>,
    /// Handlers for realising traits.
    traits: hash::Map<Option<Arc<TraitHandler>>>,
    /// Registered native attribute macro handlers.
    attribute_macros: hash::Map<Arc<AttributeMacroHandler>>,
    /// Registered types.
    types: hash::Map<ContextType>,
    /// All available names in the context.
    names: Names,
    /// Registered crates.
    crates: HashSet<Box<str>>,
    /// Constants visible in this context
    constants: hash::Map<ConstValue>,
    /// Constant constructor.
    construct: hash::Map<Arc<dyn ConstConstruct>>,
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

        // NB: Order is important, since later modules might use types defined
        // in previous modules.

        this.install(crate::modules::iter::module()?)?;
        this.install(crate::modules::core::module()?)?;
        this.install(crate::modules::cmp::module()?)?;
        this.install(crate::modules::any::module()?)?;
        this.install(crate::modules::clone::module()?)?;
        this.install(crate::modules::num::module()?)?;
        this.install(crate::modules::hash::module()?)?;

        this.install(crate::modules::string::module()?)?;
        this.install(crate::modules::bytes::module()?)?;

        this.install(crate::modules::collections::module()?)?;
        this.install(crate::modules::collections::hash_map::module()?)?;
        this.install(crate::modules::collections::hash_set::module()?)?;
        this.install(crate::modules::collections::vec_deque::module()?)?;

        this.install(crate::modules::char::module()?)?;
        this.install(crate::modules::f64::module()?)?;
        this.install(crate::modules::tuple::module()?)?;
        this.install(crate::modules::fmt::module()?)?;
        this.install(crate::modules::future::module()?)?;
        this.install(crate::modules::i64::module()?)?;
        this.install(crate::modules::u64::module()?)?;
        this.install(crate::modules::io::module(stdio)?)?;
        this.install(crate::modules::macros::module()?)?;
        this.install(crate::modules::macros::builtin::module()?)?;
        this.install(crate::modules::mem::module()?)?;
        this.install(crate::modules::object::module()?)?;
        this.install(crate::modules::ops::module()?)?;
        this.install(crate::modules::ops::generator::module()?)?;
        this.install(crate::modules::option::module()?)?;
        this.install(crate::modules::result::module()?)?;
        this.install(crate::modules::stream::module()?)?;
        this.install(crate::modules::test::module()?)?;
        this.install(crate::modules::vec::module()?)?;
        this.install(crate::modules::slice::module()?)?;
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
    /// ```no_run
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
            self.construct.try_clone()?,
        ))
    }

    /// Install the specified module.
    ///
    /// This installs everything that has been declared in the given [Module]
    /// and ensures that they are compatible with the overall context, like
    /// ensuring that a given type is only declared once.
    #[tracing::instrument(skip_all, fields(item = ?module.as_ref().item))]
    pub fn install<M>(&mut self, module: M) -> Result<(), ContextError>
    where
        M: AsRef<Module>,
    {
        let module = module.as_ref();
        tracing::trace!("installing");

        if let Some(id) = module.unique {
            if !self.unique.try_insert(id)? {
                return Ok(());
            }
        }

        if let Some(ComponentRef::Crate(name)) = module.item.first() {
            self.crates.try_insert(name.try_into()?)?;
        }

        tracing::trace!("module");
        self.install_module(module)?;

        tracing::trace!(types = module.types.len(), "types");
        for ty in &module.types {
            self.install_type(ty)?;
        }

        tracing::trace!(traits = module.traits.len(), "traits");
        for t in &module.traits {
            self.install_trait(t)?;
        }

        tracing::trace!(items = module.items.len(), "items");
        for item in &module.items {
            self.install_item(item)?;
        }

        tracing::trace!(associated = module.associated.len(), "associated");
        for assoc in &module.associated {
            self.install_associated(assoc)?;
        }

        tracing::trace!(trait_impls = module.trait_impls.len(), "trait impls");
        for t in &module.trait_impls {
            self.install_trait_impl(t)?;
        }

        tracing::trace!(reexports = module.reexports.len(), "reexports");
        for r in &module.reexports {
            self.install_reexport(r)?;
        }

        tracing::trace!(construct = module.construct.len(), "construct");
        for (hash, type_info, construct) in &module.construct {
            self.install_construct(*hash, type_info, construct)?;
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
    pub(crate) fn iter_components<'a, I>(
        &'a self,
        iter: I,
    ) -> alloc::Result<impl Iterator<Item = ComponentRef<'a>> + 'a>
    where
        I: 'a + IntoIterator,
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
    #[cfg(any(feature = "cli", feature = "languageserver", feature = "emit"))]
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

    /// Lookup deprecation by function hash.
    pub fn lookup_deprecation(&self, hash: Hash) -> Option<&str> {
        self.deprecations.get(&hash).map(|s| s.as_str())
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
    #[cfg(all(feature = "doc", feature = "cli"))]
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = Hash> + '_ {
        self.associated
            .get(&hash)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .copied()
    }

    /// Get all traits implemented for the given hash.
    #[cfg(all(feature = "doc", feature = "cli"))]
    pub(crate) fn traits(&self, hash: Hash) -> impl Iterator<Item = Hash> + '_ {
        self.implemented_traits
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
    #[cfg(feature = "cli")]
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

    /// Try to find an existing module.
    fn find_existing_module(&self, hash: Hash) -> Option<usize> {
        let indexes = self.hash_to_meta.get(&hash)?;

        for &index in indexes {
            let Some(m) = self.meta.get(index) else {
                continue;
            };

            if matches!(m.kind, meta::Kind::Module) {
                return Some(index);
            }
        }

        None
    }

    /// Install the given meta.
    #[tracing::instrument(skip_all)]
    fn install_meta(&mut self, meta: ContextMeta) -> Result<(), ContextError> {
        if let Some(item) = &meta.item {
            tracing::trace!(?item);

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
            let hash = Hash::type_hash(item);

            if let Some(index) = self.find_existing_module(hash) {
                #[cfg(feature = "doc")]
                if let Some(common) = common {
                    let meta = &mut self.meta[index];
                    meta.deprecated = common.deprecated.try_clone()?;
                    meta.docs = common.docs.try_clone()?;
                }
            } else {
                self.install_meta(ContextMeta {
                    hash,
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
            }

            current = item.parent().map(|item| (item, None));
        }

        Ok(())
    }

    /// Install a single type.
    fn install_type(&mut self, ty: &ModuleType) -> Result<(), ContextError> {
        self.install_type_info(ContextType {
            item: ty.item.try_to_owned()?,
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
                            let signature = meta::Signature {
                                #[cfg(feature = "doc")]
                                is_async: false,
                                #[cfg(feature = "doc")]
                                arguments: Some(fields_to_arguments(fields)?),
                                #[cfg(feature = "doc")]
                                return_type: meta::DocType::new(ty.hash),
                            };

                            if c.args != fields.len() {
                                return Err(ContextError::ConstructorArgumentsMismatch {
                                    type_info: ty.type_info.try_clone()?,
                                    expected: fields.len(),
                                    actual: c.args,
                                });
                            }

                            self.insert_native_fn(&ty.type_info, ty.hash, &c.handler, None)?;
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
                                        Ok(meta::FieldMeta {
                                            name: name.try_into()?,
                                            position,
                                        })
                                    })
                                    .try_collect::<alloc::Result<_>>()??,
                            }),
                            Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                            Fields::Empty => meta::Fields::Empty,
                        },
                        constructor,
                        parameters,
                        enum_hash: Hash::EMPTY,
                    }
                }
                TypeSpecification::Enum(en) => {
                    for variant in &en.variants {
                        let Some(fields) = &variant.fields else {
                            continue;
                        };

                        let kind = match fields {
                            Fields::Empty => RttiKind::Empty,
                            Fields::Unnamed(..) => RttiKind::Tuple,
                            Fields::Named(..) => RttiKind::Struct,
                        };

                        let item = ty.item.extended(variant.name)?;
                        let hash = Hash::type_hash(&item);

                        self.install_type_info(ContextType {
                            item: item.try_clone()?,
                            hash,
                            type_check: None,
                            type_info: TypeInfo::rtti(Arc::new(Rtti {
                                kind,
                                hash: ty.hash,
                                variant_hash: hash,
                                item: item.try_clone()?,
                                fields: fields.to_fields()?,
                            })),
                            type_parameters: Hash::EMPTY,
                        })?;

                        let constructor = if let Some(c) = &variant.constructor {
                            let signature = meta::Signature {
                                #[cfg(feature = "doc")]
                                is_async: false,
                                #[cfg(feature = "doc")]
                                arguments: Some(fields_to_arguments(fields)?),
                                #[cfg(feature = "doc")]
                                return_type: meta::DocType::new(ty.hash),
                            };

                            if c.args != fields.len() {
                                return Err(ContextError::VariantConstructorArgumentsMismatch {
                                    type_info: ty.type_info.try_clone()?,
                                    name: variant.name,
                                    expected: fields.len(),
                                    actual: c.args,
                                });
                            }

                            self.insert_native_fn(
                                &item,
                                hash,
                                &c.handler,
                                variant.deprecated.as_deref(),
                            )?;
                            Some(signature)
                        } else {
                            None
                        };

                        self.install_meta(ContextMeta {
                            hash,
                            item: Some(item),
                            kind: meta::Kind::Struct {
                                fields: match fields {
                                    Fields::Named(names) => {
                                        meta::Fields::Named(meta::FieldsNamed {
                                            fields: names
                                                .iter()
                                                .copied()
                                                .enumerate()
                                                .map(|(position, name)| {
                                                    Ok(meta::FieldMeta {
                                                        name: name.try_into()?,
                                                        position,
                                                    })
                                                })
                                                .try_collect::<alloc::Result<_>>()??,
                                        })
                                    }
                                    Fields::Unnamed(args) => meta::Fields::Unnamed(*args),
                                    Fields::Empty => meta::Fields::Empty,
                                },
                                constructor,
                                parameters: Hash::EMPTY,
                                enum_hash: ty.hash,
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
            item: Some(ty.item.try_to_owned()?),
            kind,
            #[cfg(feature = "doc")]
            deprecated: ty.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: ty.common.docs.try_clone()?,
        })?;

        Ok(())
    }

    fn install_trait(&mut self, t: &ModuleTrait) -> Result<(), ContextError> {
        if self.traits.try_insert(t.hash, t.handler.clone())?.is_some() {
            return Err(ContextError::ConflictingTrait {
                item: t.item.try_clone()?,
                hash: t.hash,
            });
        }

        self.install_meta(ContextMeta {
            hash: t.hash,
            item: Some(t.item.try_clone()?),
            kind: meta::Kind::Trait,
            #[cfg(feature = "doc")]
            deprecated: t.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: t.common.docs.try_clone()?,
        })?;

        for f in &t.functions {
            let signature = meta::Signature::from_context(&f.doc, &f.common)?;

            let kind = meta::Kind::Function {
                associated: Some(f.name.kind.try_clone()?),
                trait_hash: None,
                signature,
                is_test: false,
                is_bench: false,
                parameters: Hash::EMPTY.with_function_parameters(f.name.function_parameters),
                #[cfg(feature = "doc")]
                container: Some(t.hash),
                #[cfg(feature = "doc")]
                parameter_types: f.name.parameter_types.try_clone()?,
            };

            let hash = f
                .name
                .kind
                .hash(t.hash)
                .with_function_parameters(f.name.function_parameters);

            let item = if let meta::AssociatedKind::Instance(name) = &f.name.kind {
                let item = t.item.extended(name.as_ref())?;
                let hash = Hash::type_hash(&item);
                Some((hash, item))
            } else {
                None
            };

            self.install_meta(ContextMeta {
                hash,
                item: item.map(|(_, item)| item),
                kind,
                #[cfg(feature = "doc")]
                deprecated: f.common.deprecated.try_clone()?,
                #[cfg(feature = "doc")]
                docs: f.common.docs.try_clone()?,
            })?;
        }

        Ok(())
    }

    fn install_trait_impl(&mut self, i: &ModuleTraitImpl) -> Result<(), ContextError> {
        if !self.types.contains_key(&i.hash) {
            return Err(ContextError::MissingType {
                item: i.item.try_to_owned()?,
                type_info: i.type_info.try_clone()?,
            });
        };

        let Some(handler) = self.traits.get(&i.trait_hash).cloned() else {
            return Err(ContextError::MissingTrait {
                item: i.trait_item.try_clone()?,
                hash: i.hash,
                impl_item: i.item.try_to_owned()?,
                impl_hash: i.hash,
            });
        };

        if let Some(handler) = handler {
            handler(&mut TraitContext {
                cx: self,
                item: &i.item,
                hash: i.hash,
                type_info: &i.type_info,
                trait_item: &i.trait_item,
                trait_hash: i.trait_hash,
            })?;
        }

        #[cfg(feature = "doc")]
        self.implemented_traits
            .entry(i.hash)
            .or_try_default()?
            .try_push(i.trait_hash)?;

        Ok(())
    }

    fn install_reexport(&mut self, r: &ModuleReexport) -> Result<(), ContextError> {
        self.install_meta(ContextMeta {
            hash: r.hash,
            item: Some(r.item.try_clone()?),
            kind: meta::Kind::Alias(meta::Alias {
                to: r.to.try_clone()?,
            }),
            #[cfg(feature = "doc")]
            deprecated: None,
            #[cfg(feature = "doc")]
            docs: Docs::EMPTY,
        })?;

        Ok(())
    }

    /// Install a constant constructor.
    fn install_construct(
        &mut self,
        hash: Hash,
        type_info: &AnyTypeInfo,
        construct: &Arc<dyn ConstConstruct>,
    ) -> Result<(), ContextError> {
        let old = self.construct.try_insert(hash, construct.clone())?;

        if old.is_some() {
            return Err(ContextError::ConflictingConstConstruct {
                type_info: TypeInfo::from(*type_info),
                hash,
            });
        }

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
            Hash::associated_function(ty.hash, &Protocol::INTO_TYPE_NAME),
            ConstValue::from(ty.item.try_to_string()?),
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
    fn install_item(&mut self, m: &ModuleItem) -> Result<(), ContextError> {
        self.names.insert(&m.item)?;

        let kind = match &m.kind {
            rune::module::ModuleItemKind::Constant(value) => {
                self.constants.try_insert(m.hash, value.try_clone()?)?;
                meta::Kind::Const
            }
            rune::module::ModuleItemKind::Function(f) => {
                self.constants.try_insert(
                    Hash::associated_function(m.hash, &Protocol::INTO_TYPE_NAME),
                    ConstValue::from(m.item.try_to_string()?),
                )?;

                let signature = meta::Signature::from_context(&f.doc, &m.common)?;

                self.insert_native_fn(&m.item, m.hash, &f.handler, m.common.deprecated.as_deref())?;

                meta::Kind::Function {
                    associated: None,
                    trait_hash: f.trait_hash,
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
            rune::module::ModuleItemKind::Macro(macro_) => {
                self.macros.try_insert(m.hash, macro_.handler.clone())?;
                meta::Kind::Macro
            }
            rune::module::ModuleItemKind::AttributeMacro(macro_) => {
                self.attribute_macros
                    .try_insert(m.hash, macro_.handler.clone())?;
                meta::Kind::AttributeMacro
            }
        };

        self.install_meta(ContextMeta {
            hash: m.hash,
            item: Some(m.item.try_to_owned()?),
            kind,
            #[cfg(feature = "doc")]
            deprecated: m.common.deprecated.try_clone()?,
            #[cfg(feature = "doc")]
            docs: m.common.docs.try_clone()?,
        })?;

        Ok(())
    }

    fn install_associated(&mut self, assoc: &ModuleAssociated) -> Result<(), ContextError> {
        let Some(info) = self.types.get(&assoc.container).try_cloned()? else {
            return Err(ContextError::MissingContainer {
                container: assoc.container_type_info.try_clone()?,
            });
        };

        let hash = assoc
            .name
            .kind
            .hash(assoc.container)
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
                if let Some((hash, ..)) = item {
                    self.constants.try_insert(hash, value.try_clone()?)?;
                }

                self.constants.try_insert(hash, value.try_clone()?)?;
                meta::Kind::Const
            }
            ModuleAssociatedKind::Function(f) => {
                let signature = meta::Signature::from_context(&f.doc, &assoc.common)?;

                if let Some((hash, item)) = &item {
                    self.constants.try_insert(
                        Hash::associated_function(*hash, &Protocol::INTO_TYPE_NAME),
                        ConstValue::from(item.try_to_string()?),
                    )?;

                    self.insert_native_fn(
                        &assoc.container_type_info,
                        *hash,
                        &f.handler,
                        assoc.common.deprecated.as_deref(),
                    )?;
                }

                self.insert_native_fn(
                    &assoc.container_type_info,
                    hash,
                    &f.handler,
                    assoc.common.deprecated.as_deref(),
                )?;

                meta::Kind::Function {
                    associated: Some(assoc.name.kind.try_clone()?),
                    trait_hash: f.trait_hash,
                    signature,
                    is_test: false,
                    is_bench: false,
                    parameters: Hash::EMPTY
                        .with_type_parameters(info.type_parameters)
                        .with_function_parameters(assoc.name.function_parameters),
                    #[cfg(feature = "doc")]
                    container: Some(assoc.container),
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
        display: &dyn fmt::Display,
        hash: Hash,
        handler: &Arc<FunctionHandler>,
        deprecation: Option<&str>,
    ) -> Result<(), ContextError> {
        if self.functions.contains_key(&hash) {
            return Err(ContextError::ConflictingFunction {
                part: display.try_to_string()?.try_into()?,
                hash,
            });
        }

        self.functions.try_insert(hash, handler.clone())?;

        if let Some(msg) = deprecation {
            self.deprecations.try_insert(hash, msg.try_to_owned()?)?;
        }

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

impl ConstContext for Context {
    #[inline]
    fn get(&self, hash: Hash) -> Option<&dyn ConstConstruct> {
        Some(&**self.construct.get(&hash)?)
    }
}

#[cfg(feature = "doc")]
fn fields_to_arguments(fields: &Fields) -> alloc::Result<Box<[meta::DocArgument]>> {
    match *fields {
        Fields::Named(fields) => {
            let mut out = Vec::try_with_capacity(fields.len())?;

            for &name in fields {
                out.try_push(meta::DocArgument {
                    name: meta::DocName::Name(Box::try_from(name)?),
                    base: Hash::EMPTY,
                    generics: Box::default(),
                })?;
            }

            Box::try_from(out)
        }
        Fields::Unnamed(args) => {
            let mut out = Vec::try_with_capacity(args)?;

            for n in 0..args {
                out.try_push(meta::DocArgument {
                    name: meta::DocName::Index(n),
                    base: Hash::EMPTY,
                    generics: Box::default(),
                })?;
            }

            Box::try_from(out)
        }
        Fields::Empty => Ok(Box::default()),
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send, Sync);
