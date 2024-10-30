use core::marker::PhantomData;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, HashSet};
use crate::compile::context::{AttributeMacroHandler, MacroHandler};
use crate::compile::{self, meta, ContextError, Docs, Named};
use crate::function::{Async, Function, FunctionKind, InstanceFunction, Plain};
use crate::function_meta::{
    Associated, AssociatedFunctionData, AssociatedName, FunctionArgs, FunctionBuilder,
    FunctionData, FunctionMeta, FunctionMetaKind, MacroMeta, MacroMetaKind, ToFieldFunction,
    ToInstance,
};
use crate::item::IntoComponent;
use crate::macros::{MacroContext, TokenStream};
use crate::module::DocFunction;
use crate::runtime::{
    ConstConstruct, GeneratorState, InstAddress, MaybeTypeOf, Memory, Output, Protocol,
    ToConstValue, TypeCheck, TypeHash, TypeInfo, TypeOf, Value, VmResult,
};
use crate::{Hash, Item, ItemBuf};

use super::{
    AssociatedKey, EnumMut, InstallWith, InternalEnum, InternalEnumMut, ItemFnMut, ItemMut,
    ModuleAssociated, ModuleAssociatedKind, ModuleAttributeMacro, ModuleConstantBuilder,
    ModuleFunction, ModuleFunctionBuilder, ModuleItem, ModuleItemCommon, ModuleItemKind,
    ModuleMacro, ModuleMeta, ModuleRawFunctionBuilder, ModuleReexport, ModuleTrait,
    ModuleTraitImpl, ModuleType, TraitMut, TypeMut, TypeSpecification, VariantMut,
};

#[derive(Debug, TryClone, PartialEq, Eq, Hash)]
enum Name {
    /// An associated key.
    Associated(AssociatedKey),
    /// A regular item.
    Item(Hash),
    /// A macro.
    Macro(Hash),
    /// An attribute macro.
    AttributeMacro(Hash),
    /// A conflicting trait implementation.
    TraitImpl(Hash, Hash),
}

/// A [Module] that is a collection of native functions and types.
///
/// Needs to be installed into a [Context][crate::compile::Context] using
/// [Context::install][crate::compile::Context::install].
#[derive(Default)]
pub struct Module {
    /// Uniqueness checks.
    names: HashSet<Name>,
    /// A special identifier for this module, which will cause it to not conflict if installed multiple times.
    pub(crate) unique: Option<&'static str>,
    /// The name of the module.
    pub(crate) item: ItemBuf,
    /// Functions.
    pub(crate) items: Vec<ModuleItem>,
    /// Associated items.
    pub(crate) associated: Vec<ModuleAssociated>,
    /// Registered types.
    pub(crate) types: Vec<ModuleType>,
    /// Type hash to types mapping.
    pub(crate) types_hash: HashMap<Hash, usize>,
    /// A trait registered in the current module.
    pub(crate) traits: Vec<ModuleTrait>,
    /// A trait implementation registered in the current module.
    pub(crate) trait_impls: Vec<ModuleTraitImpl>,
    /// A re-export in the current module.
    pub(crate) reexports: Vec<ModuleReexport>,
    /// Constant constructors.
    pub(crate) construct: Vec<(Hash, TypeInfo, Arc<dyn ConstConstruct>)>,
    /// Defines construct hashes.
    pub(crate) construct_hash: HashSet<Hash>,
    /// Module level metadata.
    pub(crate) common: ModuleItemCommon,
}

impl Module {
    /// Create an empty module for the root path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Modify the current module to utilise a special identifier.
    ///
    /// TODO: Deprecate after next major release.
    #[doc(hidden)]
    pub fn with_unique(self, id: &'static str) -> Self {
        Self {
            unique: Some(id),
            ..self
        }
    }

    /// Construct a new module for the given item.
    pub fn with_item(iter: impl IntoIterator<Item: IntoComponent>) -> Result<Self, ContextError> {
        Ok(Self::inner_new(ItemBuf::with_item(iter)?))
    }

    /// Construct a new module for the given crate.
    pub fn with_crate(name: &str) -> Result<Self, ContextError> {
        Ok(Self::inner_new(ItemBuf::with_crate(name)?))
    }

    /// Construct a new module for the given crate.
    pub fn with_crate_item(
        name: &str,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> Result<Self, ContextError> {
        Ok(Self::inner_new(ItemBuf::with_crate_item(name, iter)?))
    }

    /// Construct a new module from the given module meta.
    pub fn from_meta(module_meta: ModuleMeta) -> Result<Self, ContextError> {
        let meta = module_meta()?;
        let mut m = Self::inner_new(meta.item);
        m.item_mut().static_docs(meta.docs)?;
        Ok(m)
    }

    fn inner_new(item: ItemBuf) -> Self {
        Self {
            names: HashSet::new(),
            unique: None,
            item,
            items: Vec::new(),
            associated: Vec::new(),
            types: Vec::new(),
            traits: Vec::new(),
            trait_impls: Vec::new(),
            types_hash: HashMap::new(),
            reexports: Vec::new(),
            construct: Vec::new(),
            construct_hash: HashSet::new(),
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
        }
    }

    /// Mutate item-level properties for this module.
    pub fn item_mut(&mut self) -> ItemMut<'_> {
        ItemMut {
            docs: &mut self.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut self.common.deprecated,
        }
    }

    /// Register a type. Registering a type is mandatory in order to register
    /// instance functions using that type.
    ///
    /// This will allow the type to be used within scripts, using the item named
    /// here.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Context, Module};
    ///
    /// #[derive(Any)]
    /// struct MyBytes {
    ///     queue: Vec<String>,
    /// }
    ///
    /// impl MyBytes {
    ///     #[rune::function]
    ///     fn len(&self) -> usize {
    ///         self.queue.len()
    ///     }
    /// }
    ///
    /// // Register `len` without registering a type.
    /// let mut m = Module::default();
    /// // Note: cannot do this until we have registered a type.
    /// m.function_meta(MyBytes::len)?;
    ///
    /// let mut context = rune::Context::new();
    /// assert!(context.install(m).is_err());
    ///
    /// // Register `len` properly.
    /// let mut m = Module::default();
    ///
    /// m.ty::<MyBytes>()?;
    /// m.function_meta(MyBytes::len)?;
    ///
    /// let mut context = Context::new();
    /// assert!(context.install(m).is_ok());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn ty<T>(&mut self) -> Result<TypeMut<'_, T>, ContextError>
    where
        T: ?Sized + TypeOf + Named + InstallWith,
    {
        let item = ItemBuf::with_item([T::BASE_NAME])?;
        let hash = T::HASH;
        let type_parameters = T::PARAMETERS;
        let type_info = T::type_info();

        if !self.names.try_insert(Name::Item(hash))? {
            return Err(ContextError::ConflictingType {
                item,
                type_info,
                hash,
            });
        }

        let index = self.types.len();
        self.types_hash.try_insert(hash, index)?;

        self.types.try_push(ModuleType {
            item,
            hash,
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
            type_parameters,
            type_info,
            spec: None,
            constructor: None,
        })?;

        T::install_with(self)?;

        let ty = self.types.last_mut().unwrap();

        Ok(TypeMut {
            docs: &mut ty.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut ty.common.deprecated,
            spec: &mut ty.spec,
            constructor: &mut ty.constructor,
            item: &ty.item,
            _marker: PhantomData,
        })
    }

    /// Accessor to modify type metadata such as documentaiton, fields, variants.
    pub fn type_meta<T>(&mut self) -> Result<TypeMut<'_, T>, ContextError>
    where
        T: ?Sized + TypeOf + Named,
    {
        let type_hash = T::HASH;

        let Some(ty) = self.types_hash.get(&type_hash).map(|&i| &mut self.types[i]) else {
            let full_name = String::try_from(T::full_name())?;

            return Err(ContextError::MissingType {
                item: ItemBuf::with_item(&[full_name])?,
                type_info: T::type_info(),
            });
        };

        Ok(TypeMut {
            docs: &mut ty.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut ty.common.deprecated,
            spec: &mut ty.spec,
            constructor: &mut ty.constructor,
            item: &ty.item,
            _marker: PhantomData,
        })
    }

    /// Register that the given type is a struct, and that it has the given
    /// compile-time metadata. This implies that each field has a
    /// [Protocol::GET] field function.
    ///
    /// This is typically not used directly, but is used automatically with the
    /// [Any][crate::Any] derive.
    #[deprecated = "Use type_meta::<T>().make_struct(fields) instead"]
    pub fn struct_meta<T>(&mut self, fields: &'static [&'static str]) -> Result<(), ContextError>
    where
        T: ?Sized + TypeOf + Named,
    {
        self.type_meta::<T>()?.make_named_struct(fields)?;
        Ok(())
    }

    /// Register enum metadata for the given type `T`. This allows an enum to be
    /// used in limited ways in Rune.
    #[deprecated = "Use type_meta::<T>().make_enum(variants) instead"]
    #[doc(hidden)]
    pub fn enum_meta<T>(
        &mut self,
        variants: &'static [&'static str],
    ) -> Result<EnumMut<'_, T>, ContextError>
    where
        T: ?Sized + TypeOf + Named,
    {
        self.type_meta::<T>()?.make_enum(variants)
    }

    /// Access variant metadata for the given type and the index of its variant.
    pub fn variant_meta<T>(&mut self, index: usize) -> Result<VariantMut<'_, T>, ContextError>
    where
        T: ?Sized + TypeOf + Named,
    {
        let type_hash = T::HASH;

        let Some(ty) = self.types_hash.get(&type_hash).map(|&i| &mut self.types[i]) else {
            let full_name = String::try_from(T::full_name())?;

            return Err(ContextError::MissingType {
                item: ItemBuf::with_item(&[full_name])?,
                type_info: T::type_info(),
            });
        };

        let Some(TypeSpecification::Enum(en)) = &mut ty.spec else {
            let full_name = String::try_from(T::full_name())?;

            return Err(ContextError::MissingEnum {
                item: ItemBuf::with_item(&[full_name])?,
                type_info: T::type_info(),
            });
        };

        let Some(variant) = en.variants.get_mut(index) else {
            return Err(ContextError::MissingVariant {
                type_info: T::type_info(),
                index,
            });
        };

        Ok(VariantMut {
            index,
            docs: &mut variant.docs,
            fields: &mut variant.fields,
            constructor: &mut variant.constructor,
            _marker: PhantomData,
        })
    }

    /// Register a variant constructor for type `T`.
    #[deprecated = "Use variant_meta() instead"]
    pub fn variant_constructor<F, A>(
        &mut self,
        index: usize,
        constructor: F,
    ) -> Result<(), ContextError>
    where
        F: Function<A, Plain>,
        F::Return: TypeOf + Named,
    {
        self.variant_meta::<F::Return>(index)?
            .constructor(constructor)?;
        Ok(())
    }

    /// Construct the type information for the `GeneratorState` type.
    ///
    /// Registering this allows the given type to be used in Rune scripts when
    /// referring to the `GeneratorState` type.
    ///
    /// # Examples
    ///
    /// This shows how to register the `GeneratorState` as
    /// `nonstd::ops::GeneratorState`.
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::with_crate_item("nonstd", ["ops"])?;
    /// module.generator_state(["GeneratorState"])?;
    ///
    /// Ok::<_, rune::support::Error>(())
    pub fn generator_state<N>(
        &mut self,
        name: N,
    ) -> Result<InternalEnumMut<'_, GeneratorState>, ContextError>
    where
        N: IntoComponent,
    {
        let mut enum_ = InternalEnum::new(
            "GeneratorState",
            crate::runtime::static_type::GENERATOR_STATE,
        );

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant(
            "Complete",
            TypeCheck::GeneratorState(0),
            GeneratorState::Complete,
        )?;

        enum_.variant(
            "Yielded",
            TypeCheck::GeneratorState(1),
            GeneratorState::Yielded,
        )?;

        self.install_internal_enum(name, enum_)
    }

    /// Construct type information for the `Option` type.
    ///
    /// Registering this allows the given type to be used in Rune scripts when
    /// referring to the `Option` type.
    ///
    /// # Examples
    ///
    /// This shows how to register the `Option` as `nonstd::option::Option`.
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::with_crate_item("nonstd", ["option"])?;
    /// module.option(["Option"])?;
    ///
    /// Ok::<_, rune::support::Error>(())
    pub fn option<N>(&mut self, name: N) -> Result<InternalEnumMut<'_, Option<Value>>, ContextError>
    where
        N: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Option", crate::runtime::static_type::OPTION);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Some", TypeCheck::Option(0), Option::<Value>::Some)?;
        enum_.variant("None", TypeCheck::Option(1), || Option::<Value>::None)?;

        self.install_internal_enum(name, enum_)
    }

    /// Construct type information for the internal `Result` type.
    ///
    /// Registering this allows the given type to be used in Rune scripts when
    /// referring to the `Result` type.
    ///
    /// # Examples
    ///
    /// This shows how to register the `Result` as `nonstd::result::Result`.
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::with_crate_item("nonstd", ["result"])?;
    /// module.result(["Result"])?;
    ///
    /// Ok::<_, rune::support::Error>(())
    pub fn result<N>(
        &mut self,
        name: N,
    ) -> Result<InternalEnumMut<'_, Result<Value, Value>>, ContextError>
    where
        N: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Result", crate::runtime::static_type::RESULT);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Ok", TypeCheck::Result(0), Result::<Value, Value>::Ok)?;
        enum_.variant("Err", TypeCheck::Result(1), Result::<Value, Value>::Err)?;

        self.install_internal_enum(name, enum_)
    }

    fn install_internal_enum<N, T>(
        &mut self,
        name: N,
        enum_: InternalEnum,
    ) -> Result<InternalEnumMut<'_, T>, ContextError>
    where
        N: IntoComponent,
        T: ?Sized + TypeOf,
    {
        let item = self.item.join([name])?;

        self.items.try_push(ModuleItem {
            item,
            common: ModuleItemCommon::default(),
            kind: ModuleItemKind::InternalEnum(enum_),
        })?;

        let item = self.items.last_mut().unwrap();

        let internal_enum = match &mut item.kind {
            ModuleItemKind::InternalEnum(internal_enum) => internal_enum,
            _ => unreachable!(),
        };

        Ok(InternalEnumMut {
            enum_: internal_enum,
            common: &mut item.common,
            _marker: PhantomData,
        })
    }

    /// Register a constant value, at a crate, module or associated level.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Module};
    ///
    /// let mut module = Module::default();
    ///
    /// #[derive(Any)]
    /// struct MyType;
    ///
    /// module.constant("TEN", 10).build()?.docs(["A global ten value."]);
    /// module.constant("TEN", 10).build_associated::<MyType>()?.docs(rune::docstring! {
    ///     /// Ten which looks like an associated constant.
    /// });
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn constant<N, V>(&mut self, name: N, value: V) -> ModuleConstantBuilder<'_, N, V>
    where
        V: TypeHash + TypeOf + ToConstValue,
    {
        ModuleConstantBuilder {
            module: self,
            name,
            value,
        }
    }

    pub(super) fn insert_constant<N, V>(
        &mut self,
        name: N,
        value: V,
    ) -> Result<ItemMut<'_>, ContextError>
    where
        N: IntoComponent,
        V: TypeHash + TypeOf + ToConstValue,
    {
        let item = self.item.join([name])?;
        let hash = Hash::type_hash(&item);

        let value = match value.to_const_value() {
            Ok(value) => value,
            Err(error) => return Err(ContextError::InvalidConstValue { item, error }),
        };

        if !self.names.try_insert(Name::Item(hash))? {
            return Err(ContextError::ConflictingConstantName { item, hash });
        }

        self.items.try_push(ModuleItem {
            item,
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
            kind: ModuleItemKind::Constant(value),
        })?;

        self.insert_const_construct::<V>()?;

        let c = self.items.last_mut().unwrap();

        Ok(ItemMut {
            docs: &mut c.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut c.common.deprecated,
        })
    }

    pub(super) fn insert_associated_constant<V>(
        &mut self,
        associated: Associated,
        value: V,
    ) -> Result<ItemMut<'_>, ContextError>
    where
        V: TypeHash + TypeOf + ToConstValue,
    {
        let value = match value.to_const_value() {
            Ok(value) => value,
            Err(error) => {
                return Err(ContextError::InvalidAssociatedConstValue {
                    container: associated.container_type_info,
                    kind: associated.name.kind,
                    error,
                })
            }
        };

        self.insert_associated_name(&associated)?;

        self.associated.try_push(ModuleAssociated {
            container: associated.container,
            container_type_info: associated.container_type_info,
            name: associated.name,
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
            kind: ModuleAssociatedKind::Constant(value),
        })?;

        self.insert_const_construct::<V>()?;

        let last = self.associated.last_mut().unwrap();

        Ok(ItemMut {
            docs: &mut last.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut last.common.deprecated,
        })
    }

    fn insert_const_construct<V>(&mut self) -> alloc::Result<()>
    where
        V: TypeHash + TypeOf + ToConstValue,
    {
        if self.construct_hash.try_insert(V::HASH)? {
            if let Some(construct) = V::construct() {
                self.construct
                    .try_push((V::HASH, V::type_info(), construct))?;
            }
        }

        Ok(())
    }

    /// Register a native macro handler through its meta.
    ///
    /// The metadata must be provided by annotating the function with
    /// [`#[rune::macro_]`][crate::macro_].
    ///
    /// This has the benefit that it captures documentation comments which can
    /// be used when generating documentation or referencing the function
    /// through code sense systems.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    /// use rune::ast;
    /// use rune::compile;
    /// use rune::macros::{quote, MacroContext, TokenStream};
    /// use rune::parse::Parser;
    /// use rune::alloc::prelude::*;
    ///
    /// /// Takes an identifier and converts it into a string.
    /// ///
    /// /// # Examples
    /// ///
    /// /// ```rune
    /// /// assert_eq!(ident_to_string!(Hello), "Hello");
    /// /// ```
    /// #[rune::macro_]
    /// fn ident_to_string(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut p = Parser::from_token_stream(stream, cx.input_span());
    ///     let ident = p.parse_all::<ast::Ident>()?;
    ///     let ident = cx.resolve(ident)?.try_to_owned()?;
    ///     let string = cx.lit(&ident)?;
    ///     Ok(quote!(#string).into_token_stream(cx)?)
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_meta(ident_to_string)?;
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn macro_meta(&mut self, meta: MacroMeta) -> Result<ItemMut<'_>, ContextError> {
        let meta = meta()?;

        let item = match meta.kind {
            MacroMetaKind::Function(data) => {
                let item = self.item.join(&data.item)?;
                let hash = Hash::type_hash(&item);

                if !self.names.try_insert(Name::Macro(hash))? {
                    return Err(ContextError::ConflictingMacroName { item, hash });
                }

                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs)?;

                self.items.try_push(ModuleItem {
                    item,
                    common: ModuleItemCommon {
                        docs,
                        deprecated: None,
                    },
                    kind: ModuleItemKind::Macro(ModuleMacro {
                        handler: data.handler,
                    }),
                })?;

                self.items.last_mut().unwrap()
            }
            MacroMetaKind::Attribute(data) => {
                let item = self.item.join(&data.item)?;
                let hash = Hash::type_hash(&item);

                if !self.names.try_insert(Name::AttributeMacro(hash))? {
                    return Err(ContextError::ConflictingMacroName { item, hash });
                }

                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs)?;

                self.items.try_push(ModuleItem {
                    item,
                    common: ModuleItemCommon {
                        docs,
                        deprecated: None,
                    },
                    kind: ModuleItemKind::AttributeMacro(ModuleAttributeMacro {
                        handler: data.handler,
                    }),
                })?;

                self.items.last_mut().unwrap()
            }
        };

        Ok(ItemMut {
            docs: &mut item.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut item.common.deprecated,
        })
    }

    /// Register a native macro handler.
    ///
    /// If possible, [`Module::macro_meta`] should be used since it includes more
    /// useful information about the macro.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    /// use rune::ast;
    /// use rune::compile;
    /// use rune::macros::{quote, MacroContext, TokenStream};
    /// use rune::parse::Parser;
    /// use rune::alloc::prelude::*;
    ///
    /// fn ident_to_string(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut p = Parser::from_token_stream(stream, cx.input_span());
    ///     let ident = p.parse_all::<ast::Ident>()?;
    ///     let ident = cx.resolve(ident)?.try_to_owned()?;
    ///     let string = cx.lit(&ident)?;
    ///     Ok(quote!(#string).into_token_stream(cx)?)
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_(["ident_to_string"], ident_to_string)?;
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn macro_<N, M>(&mut self, name: N, f: M) -> Result<ItemMut<'_>, ContextError>
    where
        M: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoComponent,
    {
        let item = self.item.join([name])?;
        let hash = Hash::type_hash(&item);

        if !self.names.try_insert(Name::Macro(hash))? {
            return Err(ContextError::ConflictingMacroName { item, hash });
        }

        let handler: Arc<MacroHandler> = Arc::new(f);

        self.items.try_push(ModuleItem {
            item,
            common: ModuleItemCommon::default(),
            kind: ModuleItemKind::Macro(ModuleMacro { handler }),
        })?;

        let m = self.items.last_mut().unwrap();

        Ok(ItemMut {
            docs: &mut m.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut m.common.deprecated,
        })
    }

    /// Register a native attribute macro handler.
    ///
    /// If possible, [`Module::macro_meta`] should be used since it includes more
    /// useful information about the function.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    /// use rune::ast;
    /// use rune::compile;
    /// use rune::macros::{quote, MacroContext, TokenStream, ToTokens};
    /// use rune::parse::Parser;
    ///
    /// fn rename_fn(cx: &mut MacroContext<'_, '_, '_>, input: &TokenStream, item: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut item = Parser::from_token_stream(item, cx.macro_span());
    ///     let mut fun = item.parse_all::<ast::ItemFn>()?;
    ///
    ///     let mut input = Parser::from_token_stream(input, cx.input_span());
    ///     fun.name = input.parse_all::<ast::EqValue<_>>()?.value;
    ///     Ok(quote!(#fun).into_token_stream(cx)?)
    /// }
    ///
    /// let mut m = Module::new();
    /// m.attribute_macro(["rename_fn"], rename_fn)?;
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn attribute_macro<N, M>(&mut self, name: N, f: M) -> Result<ItemMut<'_>, ContextError>
    where
        M: 'static
            + Send
            + Sync
            + Fn(
                &mut MacroContext<'_, '_, '_>,
                &TokenStream,
                &TokenStream,
            ) -> compile::Result<TokenStream>,
        N: IntoComponent,
    {
        let item = self.item.join([name])?;
        let hash = Hash::type_hash(&item);

        if !self.names.try_insert(Name::AttributeMacro(hash))? {
            return Err(ContextError::ConflictingMacroName { item, hash });
        }

        let handler: Arc<AttributeMacroHandler> = Arc::new(f);

        self.items.try_push(ModuleItem {
            item,
            common: ModuleItemCommon {
                docs: Docs::EMPTY,
                deprecated: None,
            },
            kind: ModuleItemKind::AttributeMacro(ModuleAttributeMacro { handler }),
        })?;

        let m = self.items.last_mut().unwrap();

        Ok(ItemMut {
            docs: &mut m.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut m.common.deprecated,
        })
    }

    /// Register a function handler through its meta.
    ///
    /// The metadata must be provided by annotating the function with
    /// [`#[rune::function]`][macro@crate::function].
    ///
    /// This has the benefit that it captures documentation comments which can
    /// be used when generating documentation or referencing the function
    /// through code sense systems.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Module, ContextError};
    /// use rune::runtime::Ref;
    ///
    /// /// This is a pretty neat function.
    /// #[rune::function]
    /// fn to_string(string: &str) -> String {
    ///     string.to_string()
    /// }
    ///
    /// /// This is a pretty neat download function
    /// #[rune::function]
    /// async fn download(url: Ref<str>) -> rune::support::Result<String> {
    ///     # todo!()
    /// }
    ///
    /// fn module() -> Result<Module, ContextError> {
    ///     let mut m = Module::new();
    ///     m.function_meta(to_string)?;
    ///     m.function_meta(download)?;
    ///     Ok(m)
    /// }
    /// ```
    ///
    /// Registering instance functions:
    ///
    /// ```
    /// use rune::{Any, Module};
    /// use rune::runtime::Ref;
    ///
    /// #[derive(Any)]
    /// struct MyBytes {
    ///     queue: Vec<String>,
    /// }
    ///
    /// impl MyBytes {
    ///     fn new() -> Self {
    ///         Self {
    ///             queue: Vec::new(),
    ///         }
    ///     }
    ///
    ///     #[rune::function]
    ///     fn len(&self) -> usize {
    ///         self.queue.len()
    ///     }
    ///
    ///     #[rune::function(instance, path = Self::download)]
    ///     async fn download(this: Ref<Self>, url: Ref<str>) -> rune::support::Result<()> {
    ///         # todo!()
    ///     }
    /// }
    ///
    /// let mut m = Module::default();
    ///
    /// m.ty::<MyBytes>()?;
    /// m.function_meta(MyBytes::len)?;
    /// m.function_meta(MyBytes::download)?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn function_meta(&mut self, meta: FunctionMeta) -> Result<ItemFnMut<'_>, ContextError> {
        let meta = meta()?;

        let mut docs = Docs::EMPTY;
        docs.set_docs(meta.statics.docs)?;
        docs.set_arguments(meta.statics.arguments)?;
        let deprecated = meta.statics.deprecated.map(TryInto::try_into).transpose()?;

        match meta.kind {
            FunctionMetaKind::Function(data) => self.function_inner(data, docs, deprecated),
            FunctionMetaKind::AssociatedFunction(data) => {
                self.insert_associated_function(data, docs, deprecated)
            }
        }
    }

    pub(super) fn function_from_meta_kind(
        &mut self,
        kind: FunctionMetaKind,
    ) -> Result<ItemFnMut<'_>, ContextError> {
        match kind {
            FunctionMetaKind::Function(data) => self.function_inner(data, Docs::EMPTY, None),
            FunctionMetaKind::AssociatedFunction(data) => {
                self.insert_associated_function(data, Docs::EMPTY, None)
            }
        }
    }

    /// Register a function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes more
    /// useful information about the function.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    ///
    /// fn add_ten(value: i64) -> i64 {
    ///     value + 10
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// module.function("add_ten", add_ten)
    ///     .build()?
    ///     .docs(["Adds 10 to any integer passed in."])?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// Asynchronous function:
    ///
    /// ```
    /// use rune::{Any, Module};
    /// # async fn download(url: &str) -> Result<String, DownloadError> { Ok(String::new()) }
    ///
    /// #[derive(Any)]
    /// struct DownloadError {
    ///     /* .. */
    /// }
    ///
    /// async fn download_quote() -> Result<String, DownloadError> {
    ///     download("https://api.quotable.io/random").await
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// module.function("download_quote", download_quote).build()?
    ///     .docs(["Download a random quote from the internet."]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn function<F, A, N, K>(&mut self, name: N, f: F) -> ModuleFunctionBuilder<'_, F, A, N, K>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        ModuleFunctionBuilder {
            module: self,
            inner: FunctionBuilder::new(name, f),
        }
    }

    /// See [`Module::function`].
    #[deprecated = "Use `Module::function`"]
    pub fn function2<F, A, N, K>(
        &mut self,
        name: N,
        f: F,
    ) -> Result<ModuleFunctionBuilder<'_, F, A, N, K>, ContextError>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(ModuleFunctionBuilder {
            module: self,
            inner: FunctionBuilder::new(name, f),
        })
    }

    /// See [`Module::function`].
    #[deprecated = "Use Module::function() instead"]
    pub fn async_function<F, A, N>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: Function<A, Async>,
        F::Return: MaybeTypeOf,
        N: IntoComponent,
        A: FunctionArgs,
    {
        self.function_inner(FunctionData::new(name, f)?, Docs::EMPTY, None)
    }

    /// Register an instance function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes
    /// more useful information about the function.
    ///
    /// This returns a [`ItemMut`], which is a handle that can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Replacing this with `function_meta` and `#[rune::function]`
    ///
    /// This is how you declare an instance function which takes `&self` or
    /// `&mut self`:
    ///
    /// ```rust
    /// # use rune::Any;
    /// #[derive(Any)]
    /// struct Struct {
    ///     /* .. */
    /// }
    ///
    /// impl Struct {
    ///     /// Get the length of the `Struct`.
    ///     #[rune::function]
    ///     fn len(&self) -> usize {
    ///         /* .. */
    ///         # todo!()
    ///     }
    /// }
    /// ```
    ///
    /// If a function does not take `&self` or `&mut self`, you must specify that
    /// it's an instance function using `#[rune::function(instance)]`. The first
    /// argument is then considered the instance the function gets associated with:
    ///
    /// ```rust
    /// # use rune::Any;
    /// #[derive(Any)]
    /// struct Struct {
    ///     /* .. */
    /// }
    ///
    /// /// Get the length of the `Struct`.
    /// #[rune::function(instance)]
    /// fn len(this: &Struct) -> usize {
    ///     /* .. */
    ///     # todo!()
    /// }
    /// ```
    ///
    /// To declare an associated function which does not receive the type we
    /// must specify the path to the function using `#[rune::function(path =
    /// Self::<name>)]`:
    ///
    /// ```rust
    /// # use rune::Any;
    /// #[derive(Any)]
    /// struct Struct {
    ///     /* .. */
    /// }
    ///
    /// impl Struct {
    ///     /// Construct a new [`Struct`].
    ///     #[rune::function(path = Self::new)]
    ///     fn new() -> Struct {
    ///         Struct {
    ///            /* .. */
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Or externally like this:
    ///
    /// ```rust
    /// # use rune::Any;
    /// #[derive(Any)]
    /// struct Struct {
    ///     /* .. */
    /// }
    ///
    /// /// Construct a new [`Struct`].
    /// #[rune::function(free, path = Struct::new)]
    /// fn new() -> Struct {
    ///     Struct {
    ///        /* .. */
    ///     }
    /// }
    /// ```
    ///
    /// The first part `Struct` in `Struct::new` is used to determine the type
    /// the function is associated with.
    ///
    /// Protocol functions can either be defined in an impl block or externally.
    /// To define a protocol externally, you can simply do this:
    ///
    /// ```rust
    /// # use rune::Any;
    /// # use rune::runtime::Formatter;
    /// #[derive(Any)]
    /// struct Struct {
    ///     /* .. */
    /// }
    ///
    /// #[rune::function(instance, protocol = STRING_DISPLAY)]
    /// fn string_display(this: &Struct, f: &mut Formatter) -> std::fmt::Result {
    ///     /* .. */
    ///     # todo!()
    /// }
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Module};
    ///
    /// #[derive(Any)]
    /// struct MyBytes {
    ///     queue: Vec<String>,
    /// }
    ///
    /// impl MyBytes {
    ///     /// Construct a new empty bytes container.
    ///     #[rune::function(path = Self::new)]
    ///     fn new() -> Self {
    ///         Self {
    ///             queue: Vec::new(),
    ///         }
    ///     }
    ///
    ///     /// Get the number of bytes.
    ///     #[rune::function]
    ///     fn len(&self) -> usize {
    ///         self.queue.len()
    ///     }
    /// }
    ///
    /// let mut m = Module::default();
    ///
    /// m.ty::<MyBytes>()?;
    /// m.function_meta(MyBytes::new)?;
    /// m.function_meta(MyBytes::len)?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// Asynchronous function:
    ///
    /// ```
    /// use std::sync::atomic::AtomicU32;
    /// use std::sync::Arc;
    ///
    /// use rune::{Any, Module};
    /// use rune::runtime::Ref;
    ///
    /// #[derive(Clone, Debug, Any)]
    /// struct Client {
    ///     value: Arc<AtomicU32>,
    /// }
    ///
    /// #[derive(Any)]
    /// struct DownloadError {
    ///     /* .. */
    /// }
    ///
    /// impl Client {
    ///     /// Download a thing.
    ///     #[rune::function(instance, path = Self::download)]
    ///     async fn download(this: Ref<Self>) -> Result<(), DownloadError> {
    ///         /* .. */
    ///         # Ok(())
    ///     }
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// module.ty::<Client>()?;
    /// module.function_meta(Client::download)?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn associated_function<N, F, A, K>(
        &mut self,
        name: N,
        f: F,
    ) -> Result<ItemFnMut<'_>, ContextError>
    where
        N: ToInstance,
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        self.insert_associated_function(
            AssociatedFunctionData::from_instance_function(name.to_instance()?, f)?,
            Docs::EMPTY,
            None,
        )
    }

    /// See [`Module::associated_function`].
    #[deprecated = "Use Module::associated_function() instead"]
    #[inline]
    pub fn inst_fn<N, F, A, K>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        N: ToInstance,
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        self.associated_function(name, f)
    }

    /// See [`Module::associated_function`].
    #[deprecated = "Use Module::associated_function() instead"]
    pub fn async_inst_fn<N, F, A>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        N: ToInstance,
        F: InstanceFunction<A, Async>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
    {
        self.associated_function(name, f)
    }

    /// Install a protocol function that interacts with the given field.
    ///
    /// This returns a [`ItemMut`], which is a handle that can be used to
    /// associate more metadata with the inserted item.
    pub fn field_function<N, F, A>(
        &mut self,
        protocol: Protocol,
        name: N,
        f: F,
    ) -> Result<ItemFnMut<'_>, ContextError>
    where
        N: ToFieldFunction,
        F: InstanceFunction<A, Plain>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
    {
        self.insert_associated_function(
            AssociatedFunctionData::from_instance_function(name.to_field_function(protocol)?, f)?,
            Docs::EMPTY,
            None,
        )
    }

    /// See [`Module::field_function`].
    #[deprecated = "Use Module::field_function() instead"]
    #[inline]
    pub fn field_fn<N, F, A>(
        &mut self,
        protocol: Protocol,
        name: N,
        f: F,
    ) -> Result<ItemFnMut<'_>, ContextError>
    where
        N: ToFieldFunction,
        F: InstanceFunction<A, Plain>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
    {
        self.field_function(protocol, name, f)
    }

    /// Install a protocol function that interacts with the given index.
    ///
    /// An index can either be a field inside a tuple, or a variant inside of an
    /// enum as configured with [Module::enum_meta].
    pub fn index_function<F, A>(
        &mut self,
        protocol: Protocol,
        index: usize,
        f: F,
    ) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: InstanceFunction<A, Plain>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
    {
        let name = AssociatedName::index(protocol, index);
        self.insert_associated_function(
            AssociatedFunctionData::from_instance_function(name, f)?,
            Docs::EMPTY,
            None,
        )
    }

    /// See [`Module::index_function`].
    #[deprecated = "Use Module::index_function() instead"]
    #[inline]
    pub fn index_fn<F, A>(
        &mut self,
        protocol: Protocol,
        index: usize,
        f: F,
    ) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: InstanceFunction<A, Plain>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
    {
        self.index_function(protocol, index, f)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    ///
    /// This returns a [`ItemMut`], which is a handle that can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    /// use rune::runtime::{Output, Memory, ToValue, VmResult, InstAddress};
    /// use rune::vm_try;
    ///
    /// fn sum(stack: &mut dyn Memory, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
    ///     let mut number = 0;
    ///
    ///     for value in vm_try!(stack.slice_at(addr, args)) {
    ///         number += vm_try!(value.as_integer());
    ///     }
    ///
    ///     out.store(stack, number);
    ///     VmResult::Ok(())
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// module.raw_function("sum", sum)
    ///     .build()?
    ///     .docs([
    ///         "Sum all numbers provided to the function."
    ///     ])?;
    ///
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn raw_function<F, N>(&mut self, name: N, f: F) -> ModuleRawFunctionBuilder<'_, N>
    where
        F: 'static + Fn(&mut dyn Memory, InstAddress, usize, Output) -> VmResult<()> + Send + Sync,
    {
        ModuleRawFunctionBuilder {
            module: self,
            name,
            handler: Arc::new(move |stack, addr, args, output| f(stack, addr, args, output)),
        }
    }

    /// See [`Module::raw_function`].
    #[deprecated = "Use `raw_function` builder instead"]
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: 'static + Fn(&mut dyn Memory, InstAddress, usize, Output) -> VmResult<()> + Send + Sync,
        N: IntoComponent,
    {
        self.raw_function(name, f).build()
    }

    fn function_inner(
        &mut self,
        data: FunctionData,
        docs: Docs,
        #[allow(unused)] deprecated: Option<Box<str>>,
    ) -> Result<ItemFnMut<'_>, ContextError> {
        let item = self.item.join(&data.item)?;
        let hash = Hash::type_hash(&item);

        if !self.names.try_insert(Name::Item(hash))? {
            return Err(ContextError::ConflictingFunctionName { item, hash });
        }

        self.items.try_push(ModuleItem {
            item,
            common: ModuleItemCommon { docs, deprecated },
            kind: ModuleItemKind::Function(ModuleFunction {
                handler: data.handler,
                trait_hash: None,
                doc: DocFunction {
                    #[cfg(feature = "doc")]
                    is_async: data.is_async,
                    #[cfg(feature = "doc")]
                    args: data.args,
                    #[cfg(feature = "doc")]
                    return_type: data.return_type,
                    #[cfg(feature = "doc")]
                    argument_types: data.argument_types,
                },
            }),
        })?;

        let last = self.items.last_mut().unwrap();

        #[cfg(feature = "doc")]
        let last_fn = match &mut last.kind {
            ModuleItemKind::Function(f) => f,
            _ => unreachable!(),
        };

        Ok(ItemFnMut {
            docs: &mut last.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut last.common.deprecated,
            #[cfg(feature = "doc")]
            is_async: &mut last_fn.doc.is_async,
            #[cfg(feature = "doc")]
            args: &mut last_fn.doc.args,
            #[cfg(feature = "doc")]
            return_type: &mut last_fn.doc.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut last_fn.doc.argument_types,
        })
    }

    /// Install an associated function.
    fn insert_associated_function(
        &mut self,
        data: AssociatedFunctionData,
        docs: Docs,
        #[allow(unused)] deprecated: Option<Box<str>>,
    ) -> Result<ItemFnMut<'_>, ContextError> {
        self.insert_associated_name(&data.associated)?;

        self.associated.try_push(ModuleAssociated {
            container: data.associated.container,
            container_type_info: data.associated.container_type_info,
            name: data.associated.name,
            common: ModuleItemCommon { docs, deprecated },
            kind: ModuleAssociatedKind::Function(ModuleFunction {
                handler: data.handler,
                trait_hash: None,
                doc: DocFunction {
                    #[cfg(feature = "doc")]
                    is_async: data.is_async,
                    #[cfg(feature = "doc")]
                    args: data.args,
                    #[cfg(feature = "doc")]
                    return_type: data.return_type,
                    #[cfg(feature = "doc")]
                    argument_types: data.argument_types,
                },
            }),
        })?;

        let last = self.associated.last_mut().unwrap();

        #[cfg(feature = "doc")]
        let last_fn = match &mut last.kind {
            ModuleAssociatedKind::Function(f) => f,
            _ => unreachable!(),
        };

        Ok(ItemFnMut {
            docs: &mut last.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut last.common.deprecated,
            #[cfg(feature = "doc")]
            is_async: &mut last_fn.doc.is_async,
            #[cfg(feature = "doc")]
            args: &mut last_fn.doc.args,
            #[cfg(feature = "doc")]
            return_type: &mut last_fn.doc.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut last_fn.doc.argument_types,
        })
    }

    fn insert_associated_name(&mut self, associated: &Associated) -> Result<(), ContextError> {
        if !self
            .names
            .try_insert(Name::Associated(associated.as_key()?))?
        {
            return Err(match &associated.name.kind {
                meta::AssociatedKind::Protocol(protocol) => {
                    ContextError::ConflictingProtocolFunction {
                        type_info: associated.container_type_info.try_clone()?,
                        name: protocol.name.try_into()?,
                    }
                }
                meta::AssociatedKind::FieldFn(protocol, field) => {
                    ContextError::ConflictingFieldFunction {
                        type_info: associated.container_type_info.try_clone()?,
                        name: protocol.name.try_into()?,
                        field: field.as_ref().try_into()?,
                    }
                }
                meta::AssociatedKind::IndexFn(protocol, index) => {
                    ContextError::ConflictingIndexFunction {
                        type_info: associated.container_type_info.try_clone()?,
                        name: protocol.name.try_into()?,
                        index: *index,
                    }
                }
                meta::AssociatedKind::Instance(name) => ContextError::ConflictingInstanceFunction {
                    type_info: associated.container_type_info.try_clone()?,
                    name: name.as_ref().try_into()?,
                },
            });
        }

        Ok(())
    }

    /// Define a new trait.
    pub fn define_trait(
        &mut self,
        item: impl IntoIterator<Item: IntoComponent>,
    ) -> Result<TraitMut<'_>, ContextError> {
        let item = self.item.join(item)?;
        let hash = Hash::type_hash(&item);

        if !self.names.try_insert(Name::Item(hash))? {
            return Err(ContextError::ConflictingTrait { item, hash });
        }

        self.traits.try_push(ModuleTrait {
            item,
            hash,
            common: ModuleItemCommon::default(),
            handler: None,
            functions: Vec::new(),
        })?;

        let t = self.traits.last_mut().unwrap();

        Ok(TraitMut {
            docs: &mut t.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut t.common.deprecated,
            handler: &mut t.handler,
            functions: &mut t.functions,
        })
    }

    /// Implement the trait `trait_item` for the type `T`.
    pub fn implement_trait<T>(&mut self, trait_item: &Item) -> Result<(), ContextError>
    where
        T: ?Sized + TypeOf + Named,
    {
        let item = ItemBuf::with_item([T::BASE_NAME])?;
        let hash = T::HASH;
        let type_info = T::type_info();
        let trait_hash = Hash::type_hash(trait_item);

        if !self.names.try_insert(Name::TraitImpl(hash, trait_hash))? {
            return Err(ContextError::ConflictingTraitImpl {
                trait_item: trait_item.try_to_owned()?,
                trait_hash,
                item,
                hash,
            });
        }

        self.trait_impls.try_push(ModuleTraitImpl {
            item,
            hash,
            type_info,
            trait_item: trait_item.try_to_owned()?,
            trait_hash,
        })?;

        Ok(())
    }

    /// Define a re-export.
    pub fn reexport(
        &mut self,
        item: impl IntoIterator<Item: IntoComponent>,
        to: &Item,
    ) -> Result<(), ContextError> {
        let item = self.item.join(item)?;
        let hash = Hash::type_hash(&item);

        if !self.names.try_insert(Name::Item(hash))? {
            return Err(ContextError::ConflictingReexport {
                item,
                hash,
                to: to.try_to_owned()?,
            });
        }

        self.reexports.try_push(ModuleReexport {
            item,
            hash,
            to: to.try_to_owned()?,
        })?;

        Ok(())
    }
}

impl AsRef<Module> for Module {
    #[inline]
    fn as_ref(&self) -> &Module {
        self
    }
}
