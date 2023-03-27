use std::fmt;
use std::sync::Arc;

use crate::collections::{hash_map, HashMap, HashSet};
use crate::compile::module::{
    AssocFn, AssocKey, AssocKind, Function, InternalEnum, Macro, Module, ModuleFn, Type,
    TypeSpecification, UnitType, VariantKind,
};
use crate::compile::{
    ComponentRef, ContextError, ContextMeta, ContextMetaKind, ContextSignature, ContextTypeInfo,
    Docs, IntoComponent, Item, ItemBuf, Names, PrivStructMeta, PrivTupleMeta, PrivVariantMeta,
};
use crate::runtime::{
    ConstValue, FunctionHandler, MacroHandler, Protocol, RuntimeContext, StaticType, TypeCheck,
    TypeInfo, TypeOf, VariantRtti,
};
use crate::{Hash, InstFnKind};

/// Information on a specific type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PrivTypeInfo {
    /// The type check used for the current type.
    type_check: Option<TypeCheck>,
    /// Complete detailed information on the hash.
    type_info: TypeInfo,
    /// The name of the type.
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
    meta: HashMap<ItemBuf, ContextMeta>,
    /// Registered native function handlers.
    functions: HashMap<Hash, Arc<FunctionHandler>>,
    /// Registered native macro handlers.
    macros: HashMap<Hash, Arc<MacroHandler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, ContextSignature>,
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
    pub fn with_config(stdio: bool) -> Result<Self, ContextError> {
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
    /// # fn main() -> rune::Result<()> {
    /// let context = Context::with_default_modules()?;
    ///
    /// let runtime = Arc::new(context.runtime());
    /// let unit = Arc::new(Unit::default());
    ///
    /// let vm = Vm::new(runtime, unit);
    /// # Ok(()) }
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

        for (key, inst) in &module.associated_functions {
            self.install_associated_function(module, key, inst)?;
        }

        Ok(())
    }

    /// Iterate over all available functions in the [Context].
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &ContextSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Iterate over all available types in the [Context].
    pub fn iter_types(&self) -> impl Iterator<Item = (Hash, ContextTypeInfo<'_>)> {
        let mut it = self.types.iter();

        std::iter::from_fn(move || {
            let (hash, ty) = it.next()?;

            let ty = ContextTypeInfo { item: &ty.item };

            Some((*hash, ty))
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
    pub(crate) fn lookup_meta(&self, name: &Item) -> Option<&ContextMeta> {
        self.meta.get(name)
    }

    /// Look up signature of function.
    pub(crate) fn lookup_signature(&self, hash: Hash) -> Option<&ContextSignature> {
        self.functions_info.get(&hash)
    }

    /// Iterate over all metadata in the context.
    pub fn iter_meta(&self) -> impl Iterator<Item = (&Item, &ContextMeta)> + '_ {
        self.meta.iter().map(|(item, meta)| (item.as_ref(), meta))
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.names.contains_prefix(item)
    }

    /// Lookup the given native function handler in the context.
    pub(crate) fn lookup_function(&self, hash: Hash) -> Option<&Arc<FunctionHandler>> {
        self.functions.get(&hash)
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
        match self.meta.entry(meta.item.clone()) {
            hash_map::Entry::Occupied(e) => {
                return Err(ContextError::ConflictingMeta {
                    existing: Box::new(e.get().info()),
                    current: Box::new(meta.info()),
                });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(meta);
            }
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
                TypeSpecification::Struct(st) => ContextMetaKind::Struct {
                    type_hash,
                    variant: PrivVariantMeta::Struct(PrivStructMeta {
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

                        let (variant, args) = match &variant.kind {
                            VariantKind::Tuple(t) => (
                                PrivVariantMeta::Tuple(PrivTupleMeta { args: t.args, hash }),
                                Some(t.args),
                            ),
                            VariantKind::Struct(st) => (
                                PrivVariantMeta::Struct(PrivStructMeta {
                                    fields: st.fields.clone(),
                                }),
                                None,
                            ),
                            VariantKind::Unit => (PrivVariantMeta::Unit, Some(0)),
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
                            let signature = ContextSignature::Function {
                                type_hash: hash,
                                item: item.clone(),
                                args: Some(args),
                            };

                            if let Some(old) = self.functions_info.insert(hash, signature) {
                                return Err(ContextError::ConflictingFunction {
                                    signature: Box::new(old),
                                    hash,
                                });
                            }

                            self.functions.insert(hash, c.clone());
                        }

                        let kind = ContextMetaKind::Variant {
                            type_hash: hash,
                            enum_item: enum_item.clone(),
                            enum_hash,
                            index,
                            variant,
                        };

                        self.install_meta(ContextMeta::new(module, item, kind, Docs::default()))?;
                    }

                    ContextMetaKind::Enum { type_hash }
                }
            }
        } else {
            ContextMetaKind::Unknown { type_hash }
        };

        self.install_meta(ContextMeta::new(module, item, kind, docs))?;
        Ok(())
    }

    fn install_type_info(&mut self, hash: Hash, info: PrivTypeInfo) -> Result<(), ContextError> {
        self.names.insert(&info.item);

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
        f: &ModuleFn,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);
        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(item.to_string()),
        );

        let signature = ContextSignature::Function {
            type_hash: hash,
            item: item.clone(),
            args: f.args,
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: Box::new(old),
                hash,
            });
        }

        self.functions.insert(hash, f.handler.clone());

        self.install_meta(ContextMeta::new(
            module,
            item,
            ContextMetaKind::Function {
                type_hash: hash,
                args: f.args,
                instance_function: f.instance_function,
            },
            f.docs.clone(),
        ))?;

        Ok(())
    }

    /// Install a function and check for duplicates.
    fn install_macro(
        &mut self,
        module: &Module,
        item: &Item,
        m: &Macro,
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
        docs: Docs,
    ) -> Result<(), ContextError> {
        let item = module.item.join(item);

        self.names.insert(&item);

        let hash = Hash::type_hash(&item);

        self.constants.insert(hash, v.clone());

        self.install_meta(ContextMeta::new(
            module,
            item,
            ContextMetaKind::Const {
                const_value: v.clone(),
            },
            docs,
        ))?;

        Ok(())
    }

    fn install_associated_function(
        &mut self,
        module: &Module,
        key: &AssocKey,
        assoc: &AssocFn,
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

        let hash = key
            .kind
            .hash(key.type_hash, key.hash)
            .with_parameters(key.parameters);

        let signature = ContextSignature::Instance {
            type_hash: key.type_hash,
            item: info.item.clone(),
            name: assoc.name.clone(),
            args: assoc.args,
            self_type_info: info.type_info.clone(),
        };

        if let Some(old) = self.functions_info.insert(hash, signature) {
            return Err(ContextError::ConflictingFunction {
                signature: Box::new(old),
                hash,
            });
        }

        self.functions.insert(hash, assoc.handler.clone());

        // If the associated function is a named instance function - register it
        // under the name of the item it corresponds to unless it's a field
        // function.
        //
        // The other alternatives are protocol functions (which are not free)
        // and plain hashes.
        if let (InstFnKind::Instance(name), AssocKind::Instance) = (&assoc.name, key.kind) {
            let item = info.item.extended(name);
            self.names.insert(&item);

            let type_hash = Hash::type_hash(&item);
            let hash = type_hash.with_parameters(key.parameters);

            self.constants.insert(
                Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
                ConstValue::String(item.to_string()),
            );

            let signature = ContextSignature::Function {
                type_hash: hash,
                item: item.clone(),
                args: assoc.args,
            };

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: Box::new(old),
                    hash,
                });
            }

            if !self.meta.contains_key(&item) {
                self.install_meta(ContextMeta::new(
                    module,
                    item,
                    ContextMetaKind::Function {
                        type_hash,
                        args: assoc.args,
                        instance_function: true,
                    },
                    assoc.docs.clone(),
                ))?;
            }

            self.functions.insert(hash, assoc.handler.clone());
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
        self.add_internal_tuple(module, None, item.clone(), 0, || (), docs)?;

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
            module,
            enum_item.clone(),
            ContextMetaKind::Enum {
                type_hash: internal_enum.static_type.hash,
            },
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
                module,
                item.clone(),
                ContextMetaKind::Variant {
                    type_hash: hash,
                    enum_item: enum_item.clone(),
                    enum_hash,
                    index,
                    variant: PrivVariantMeta::Tuple(PrivTupleMeta {
                        args: variant.args,
                        hash,
                    }),
                },
                Docs::default(),
            ))?;

            let signature = ContextSignature::Function {
                type_hash: hash,
                item,
                args: Some(variant.args),
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
    fn add_internal_tuple<C, Args>(
        &mut self,
        module: &Module,
        enum_item: Option<(ItemBuf, Hash, usize)>,
        item: ItemBuf,
        args: usize,
        constructor: C,
        docs: Docs,
    ) -> Result<(), ContextError>
    where
        C: Function<Args>,
        C::Return: TypeOf,
    {
        let type_hash = <C::Return as TypeOf>::type_hash();
        let hash = Hash::type_hash(&item);

        let tuple = PrivTupleMeta { args, hash };

        let meta = match enum_item {
            Some((enum_item, enum_hash, index)) => ContextMeta::new(
                module,
                item.clone(),
                ContextMetaKind::Variant {
                    type_hash,
                    enum_item,
                    enum_hash,
                    index,
                    variant: PrivVariantMeta::Tuple(tuple),
                },
                docs,
            ),
            None => ContextMeta {
                #[cfg(feature = "doc")]
                module: module.item.clone(),
                item: item.clone(),
                kind: ContextMetaKind::Struct {
                    type_hash,
                    variant: PrivVariantMeta::Tuple(tuple),
                },
                docs,
            },
        };

        self.install_meta(meta)?;

        let constructor: Arc<FunctionHandler> =
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
