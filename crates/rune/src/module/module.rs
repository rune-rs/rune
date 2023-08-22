use core::marker::PhantomData;

use crate::no_std::collections::{HashMap, HashSet};
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{self, meta, ContextError, Docs, IntoComponent, ItemBuf, Named};
use crate::macros::{MacroContext, TokenStream};
use crate::module::function_meta::{
    AssociatedFunctionData, AssociatedFunctionName, FunctionArgs, FunctionData, FunctionMeta,
    FunctionMetaKind, MacroMeta, MacroMetaKind, ToFieldFunction, ToInstance,
};
use crate::module::{
    AssociatedKey, Async, EnumMut, Function, FunctionKind, InstallWith, InstanceFunction,
    InternalEnum, InternalEnumMut, ItemFnMut, ItemMut, ModuleAssociated, ModuleAttributeMacro,
    ModuleConstant, ModuleFunction, ModuleMacro, ModuleType, Plain, TypeMut, TypeSpecification,
    VariantMut,
};
use crate::runtime::{
    AttributeMacroHandler, ConstValue, FromValue, GeneratorState, MacroHandler, MaybeTypeOf,
    Protocol, Stack, ToValue, TypeCheck, TypeOf, Value, VmResult,
};
use crate::Hash;

#[doc(hidden)]
pub struct ModuleMetaData {
    #[doc(hidden)]
    pub item: ItemBuf,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
}

/// Type used to collect and store module metadata through the `#[rune::module]`
/// macro.
///
/// This is the argument type for [`Module::from_meta`], and is from a public
/// API perspective completely opaque and might change for any release.
///
/// Calling and making use of `ModuleMeta` manually despite this warning might
/// lead to future breakage.
pub type ModuleMeta = fn() -> ModuleMetaData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Name {
    /// An associated key.
    Associated(AssociatedKey),
    /// A regular item.
    Item(Hash),
    /// A macro.
    Macro(Hash),
    /// An attribute macro.
    AttributeMacro(Hash),
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
    pub(crate) functions: Vec<ModuleFunction>,
    /// MacroHandler handlers.
    pub(crate) macros: Vec<ModuleMacro>,
    /// AttributeMacroHandler handlers.
    pub(crate) attribute_macros: Vec<ModuleAttributeMacro>,
    /// Constant values.
    pub(crate) constants: Vec<ModuleConstant>,
    /// Associated items.
    pub(crate) associated: Vec<ModuleAssociated>,
    /// Registered types.
    pub(crate) types: Vec<ModuleType>,
    /// Type hash to types mapping.
    pub(crate) types_hash: HashMap<Hash, usize>,
    /// Registered generator state type.
    pub(crate) internal_enums: Vec<InternalEnum>,
    /// Module level documentation.
    pub(crate) docs: Docs,
}

impl Module {
    /// Create an empty module for the root path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Modify the current module to utilise a special identifier.
    #[doc(hidden)]
    pub fn with_unique(self, id: &'static str) -> Self {
        Self {
            unique: Some(id),
            ..self
        }
    }

    /// Construct a new module for the given item.
    pub fn with_item<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Self::inner_new(ItemBuf::with_item(iter))
    }

    /// Construct a new module for the given crate.
    pub fn with_crate(name: &str) -> Self {
        Self::inner_new(ItemBuf::with_crate(name))
    }

    /// Construct a new module for the given crate.
    pub fn with_crate_item<I>(name: &str, iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Self::inner_new(ItemBuf::with_crate_item(name, iter))
    }

    /// Construct a new module from the given module meta.
    pub fn from_meta(module_meta: ModuleMeta) -> Self {
        let meta = module_meta();
        let mut m = Self::inner_new(meta.item);
        m.item_mut().static_docs(meta.docs);
        m
    }

    fn inner_new(item: ItemBuf) -> Self {
        Self {
            names: HashSet::new(),
            unique: None,
            item,
            functions: Vec::new(),
            macros: Vec::new(),
            attribute_macros: Vec::new(),
            associated: Vec::new(),
            types: Vec::new(),
            types_hash: HashMap::new(),
            internal_enums: Vec::new(),
            constants: Vec::new(),
            docs: Docs::EMPTY,
        }
    }

    /// Mutate item-level properties for this module.
    pub fn item_mut(&mut self) -> ItemMut<'_> {
        ItemMut {
            docs: &mut self.docs,
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
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn ty<T>(&mut self) -> Result<TypeMut<'_, T>, ContextError>
    where
        T: ?Sized + Named + TypeOf + InstallWith,
    {
        let item = ItemBuf::with_item([T::BASE_NAME]);
        let hash = T::type_hash();
        let type_parameters = T::type_parameters();
        let type_info = T::type_info();

        if !self.names.insert(Name::Item(hash)) {
            return Err(ContextError::ConflictingType {
                item,
                type_info,
                hash,
            });
        }

        let index = self.types.len();
        self.types_hash.insert(hash, index);

        self.types.push(ModuleType {
            item,
            hash,
            type_parameters,
            type_info,
            spec: None,
            constructor: None,
            docs: Docs::EMPTY,
        });

        T::install_with(self)?;

        let ty = self.types.last_mut().unwrap();

        Ok(TypeMut {
            docs: &mut ty.docs,
            spec: &mut ty.spec,
            constructor: &mut ty.constructor,
            item: &ty.item,
            _marker: PhantomData,
        })
    }

    /// Accessor to modify type metadata such as documentaiton, fields, variants.
    pub fn type_meta<T>(&mut self) -> Result<TypeMut<'_, T>, ContextError>
    where
        T: Named + TypeOf,
    {
        let type_hash = T::type_hash();

        let Some(ty) = self.types_hash.get(&type_hash).map(|&i| &mut self.types[i]) else {
            return Err(ContextError::MissingType {
                item: ItemBuf::with_item(&[T::full_name()]),
                type_info: T::type_info(),
            });
        };

        Ok(TypeMut {
            docs: &mut ty.docs,
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
        T: Named + TypeOf,
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
        T: Named + TypeOf,
    {
        self.type_meta::<T>()?.make_enum(variants)
    }

    /// Access variant metadata for the given type and the index of its variant.
    pub fn variant_meta<T>(&mut self, index: usize) -> Result<VariantMut<'_, T>, ContextError>
    where
        T: Named + TypeOf,
    {
        let type_hash = T::type_hash();

        let Some(ty) = self.types_hash.get(&type_hash).map(|&i| &mut self.types[i]) else {
            return Err(ContextError::MissingType {
                item: ItemBuf::with_item(&[T::full_name()]),
                type_info: T::type_info(),
            });
        };

        let Some(TypeSpecification::Enum(en)) = &mut ty.spec else {
            return Err(ContextError::MissingEnum {
                item: ItemBuf::with_item(&[T::full_name()]),
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
        F::Return: Named + TypeOf,
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
    /// `nonstd::generator::GeneratorState`.
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::with_crate_item("nonstd", ["generator"]);
    /// module.generator_state(["GeneratorState"])?;
    /// # Ok::<_, rune::Error>(())
    pub fn generator_state<N>(
        &mut self,
        name: N,
    ) -> Result<InternalEnumMut<'_, GeneratorState>, ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ = InternalEnum::new(
            "GeneratorState",
            name,
            crate::runtime::static_type::GENERATOR_STATE_TYPE,
        );

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant(
            "Complete",
            TypeCheck::GeneratorState(0),
            GeneratorState::Complete,
        );

        enum_.variant(
            "Yielded",
            TypeCheck::GeneratorState(1),
            GeneratorState::Yielded,
        );

        self.internal_enums.push(enum_);

        Ok(InternalEnumMut {
            enum_: self.internal_enums.last_mut().unwrap(),
            _marker: PhantomData,
        })
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
    /// let mut module = Module::with_crate_item("nonstd", ["option"]);
    /// module.option(["Option"])?;
    /// # Ok::<_, rune::Error>(())
    pub fn option<N>(&mut self, name: N) -> Result<InternalEnumMut<'_, Option<Value>>, ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Option", name, crate::runtime::static_type::OPTION_TYPE);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Some", TypeCheck::Option(0), Option::<Value>::Some);
        enum_.variant("None", TypeCheck::Option(1), || Option::<Value>::None);
        self.internal_enums.push(enum_);

        Ok(InternalEnumMut {
            enum_: self.internal_enums.last_mut().unwrap(),
            _marker: PhantomData,
        })
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
    /// let mut module = Module::with_crate_item("nonstd", ["result"]);
    /// module.result(["Result"])?;
    /// # Ok::<_, rune::Error>(())
    pub fn result<N>(
        &mut self,
        name: N,
    ) -> Result<InternalEnumMut<'_, Result<Value, Value>>, ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Result", name, crate::runtime::static_type::RESULT_TYPE);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Ok", TypeCheck::Result(0), Result::<Value, Value>::Ok);
        enum_.variant("Err", TypeCheck::Result(1), Result::<Value, Value>::Err);
        self.internal_enums.push(enum_);

        Ok(InternalEnumMut {
            enum_: self.internal_enums.last_mut().unwrap(),
            _marker: PhantomData,
        })
    }

    /// Register a constant value, at a crate, module or associated level.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::default();
    ///
    /// module.constant(["TEN"], 10)?.docs(["A global ten value."]);
    /// module.constant(["MyType", "TEN"], 10)?.docs(["Ten which looks like an associated constant."]);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn constant<N, V>(&mut self, name: N, value: V) -> Result<ItemMut<'_>, ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        V: ToValue,
    {
        let item = ItemBuf::with_item(name);
        let hash = Hash::type_hash(&item);

        let value = match value.to_value() {
            VmResult::Ok(v) => v,
            VmResult::Err(error) => return Err(ContextError::ValueError { error }),
        };

        let value = match <ConstValue as FromValue>::from_value(value) {
            VmResult::Ok(v) => v,
            VmResult::Err(error) => return Err(ContextError::ValueError { error }),
        };

        if !self.names.insert(Name::Item(hash)) {
            return Err(ContextError::ConflictingConstantName { item, hash });
        }

        self.constants.push(ModuleConstant {
            item,
            value,
            docs: Docs::EMPTY,
        });

        let c = self.constants.last_mut().unwrap();
        Ok(ItemMut { docs: &mut c.docs })
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
    ///     let ident = cx.resolve(ident)?.to_owned();
    ///     let string = cx.lit(&ident);
    ///     Ok(quote!(#string).into_token_stream(cx))
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_meta(ident_to_string)?;
    /// Ok::<_, rune::Error>(())
    /// ```
    #[inline]
    pub fn macro_meta(&mut self, meta: MacroMeta) -> Result<ItemMut<'_>, ContextError> {
        let meta = meta();

        let docs = match meta.kind {
            MacroMetaKind::Function(data) => {
                let hash = Hash::type_hash(&data.item);

                if !self.names.insert(Name::Macro(hash)) {
                    return Err(ContextError::ConflictingMacroName {
                        item: data.item,
                        hash,
                    });
                }

                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs);

                self.macros.push(ModuleMacro {
                    item: data.item,
                    handler: data.handler,
                    docs,
                });
                &mut self.macros.last_mut().unwrap().docs
            }
            MacroMetaKind::Attribute(data) => {
                let hash = Hash::type_hash(&data.item);

                if !self.names.insert(Name::AttributeMacro(hash)) {
                    return Err(ContextError::ConflictingMacroName {
                        item: data.item,
                        hash,
                    });
                }

                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs);

                self.attribute_macros.push(ModuleAttributeMacro {
                    item: data.item,
                    handler: data.handler,
                    docs,
                });
                &mut self.attribute_macros.last_mut().unwrap().docs
            }
        };

        Ok(ItemMut { docs })
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
    ///
    /// fn ident_to_string(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut p = Parser::from_token_stream(stream, cx.input_span());
    ///     let ident = p.parse_all::<ast::Ident>()?;
    ///     let ident = cx.resolve(ident)?.to_owned();
    ///     let string = cx.lit(&ident);
    ///     Ok(quote!(#string).into_token_stream(cx))
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_(["ident_to_string"], ident_to_string)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn macro_<N, M>(&mut self, name: N, f: M) -> Result<ItemMut<'_>, ContextError>
    where
        M: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let item = ItemBuf::with_item(name);
        let hash = Hash::type_hash(&item);

        if !self.names.insert(Name::Macro(hash)) {
            return Err(ContextError::ConflictingMacroName { item, hash });
        }

        let handler: Arc<MacroHandler> = Arc::new(f);

        self.macros.push(ModuleMacro {
            item,
            handler,
            docs: Docs::EMPTY,
        });

        let m = self.macros.last_mut().unwrap();

        Ok(ItemMut { docs: &mut m.docs })
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
    ///     Ok(quote!(#fun).into_token_stream(cx))
    /// }
    ///
    /// let mut m = Module::new();
    /// m.attribute_macro(["rename_fn"], rename_fn)?;
    /// # Ok::<_, rune::Error>(())
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
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let item = ItemBuf::with_item(name);
        let hash = Hash::type_hash(&item);

        if !self.names.insert(Name::AttributeMacro(hash)) {
            return Err(ContextError::ConflictingMacroName { item, hash });
        }

        let handler: Arc<AttributeMacroHandler> = Arc::new(f);

        self.attribute_macros.push(ModuleAttributeMacro {
            item,
            handler,
            docs: Docs::EMPTY,
        });

        let m = self.attribute_macros.last_mut().unwrap();

        Ok(ItemMut { docs: &mut m.docs })
    }

    /// Register a function handler through its meta.
    ///
    /// The metadata must be provided by annotating the function with
    /// [`#[rune::function]`][crate::function].
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
    /// async fn download(url: Ref<str>) -> rune::Result<String> {
    ///     todo!()
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
    ///     async fn download(this: Ref<Self>, url: Ref<str>) -> rune::Result<()> {
    ///         todo!()
    ///     }
    /// }
    ///
    /// let mut m = Module::default();
    ///
    /// m.ty::<MyBytes>()?;
    /// m.function_meta(MyBytes::len)?;
    /// m.function_meta(MyBytes::download)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    #[inline]
    pub fn function_meta(&mut self, meta: FunctionMeta) -> Result<ItemFnMut<'_>, ContextError> {
        let meta = meta();

        match meta.kind {
            FunctionMetaKind::Function(data) => {
                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs);
                docs.set_arguments(meta.arguments);
                self.function_inner(data, docs)
            }
            FunctionMetaKind::AssociatedFunction(data) => {
                let mut docs = Docs::EMPTY;
                docs.set_docs(meta.docs);
                docs.set_arguments(meta.arguments);
                self.assoc_fn(data, docs)
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
    /// module.function(["add_ten"], add_ten)?.docs(["Adds 10 to any integer passed in."]);
    /// # Ok::<_, rune::Error>(())
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
    /// module.function(["download_quote"], download_quote)?
    ///     .docs(["Download a random quote from the internet."]);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn function<F, A, N, K>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        A: FunctionArgs,
        K: FunctionKind,
    {
        self.function_inner(FunctionData::new(name, f), Docs::EMPTY)
    }

    /// See [`Module::function`].
    #[deprecated = "Use Module::function() instead"]
    pub fn async_function<F, A, N>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: Function<A, Async>,
        F::Return: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        A: FunctionArgs,
    {
        self.function_inner(FunctionData::new(name, f), Docs::EMPTY)
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
    /// #[rune::function(path = Struct::new)]
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
    /// # Ok::<_, rune::Error>(())
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
    ///     async fn download(this: Ref<Self>) -> Result<(), DownloadError> {
    ///         /* .. */
    ///         # Ok(())
    ///     }
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// module.ty::<Client>()?;
    /// module.associated_function("download", Client::download)?
    ///     .docs(["Download a thing."]);
    /// # Ok::<_, rune::Error>(())
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
        self.assoc_fn(
            AssociatedFunctionData::new(name.to_instance(), f),
            Docs::EMPTY,
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
        self.assoc_fn(
            AssociatedFunctionData::new(name.to_field_function(protocol), f),
            Docs::EMPTY,
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
        let name = AssociatedFunctionName::index(protocol, index);
        self.assoc_fn(AssociatedFunctionData::new(name, f), Docs::EMPTY)
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
    /// use rune::runtime::{Stack, VmResult};
    /// use rune::vm_try;
    ///
    /// fn sum(stack: &mut Stack, args: usize) -> VmResult<()> {
    ///     let mut number = 0;
    ///
    ///     for _ in 0..args {
    ///         number += vm_try!(vm_try!(stack.pop()).into_integer());
    ///     }
    ///
    ///     stack.push(number);
    ///     VmResult::Ok(())
    /// }
    ///
    /// let mut module = Module::default();
    ///
    /// let sum = module.raw_fn(["sum"], sum)?;
    /// sum.docs([
    ///     "Sum all numbers provided to the function."
    /// ]);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<ItemFnMut<'_>, ContextError>
    where
        F: 'static + Fn(&mut Stack, usize) -> VmResult<()> + Send + Sync,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let item = ItemBuf::with_item(name);
        let hash = Hash::type_hash(&item);

        if !self.names.insert(Name::Item(hash)) {
            return Err(ContextError::ConflictingFunctionName { item, hash });
        }

        self.functions.push(ModuleFunction {
            item,
            handler: Arc::new(move |stack, args| f(stack, args)),
            #[cfg(feature = "doc")]
            is_async: false,
            #[cfg(feature = "doc")]
            deprecated: None,
            #[cfg(feature = "doc")]
            args: None,
            #[cfg(feature = "doc")]
            return_type: None,
            #[cfg(feature = "doc")]
            argument_types: Box::from([]),
            docs: Docs::EMPTY,
        });

        let last = self.functions.last_mut().unwrap();

        Ok(ItemFnMut {
            docs: &mut last.docs,
            #[cfg(feature = "doc")]
            is_async: &mut last.is_async,
            #[cfg(feature = "doc")]
            deprecated: &mut last.deprecated,
            #[cfg(feature = "doc")]
            args: &mut last.args,
            #[cfg(feature = "doc")]
            return_type: &mut last.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut last.argument_types,
        })
    }

    fn function_inner(
        &mut self,
        data: FunctionData,
        docs: Docs,
    ) -> Result<ItemFnMut<'_>, ContextError> {
        let hash = Hash::type_hash(&data.item);

        if !self.names.insert(Name::Item(hash)) {
            return Err(ContextError::ConflictingFunctionName {
                item: data.item,
                hash,
            });
        }

        self.functions.push(ModuleFunction {
            item: data.item,
            handler: data.handler,
            #[cfg(feature = "doc")]
            is_async: data.is_async,
            #[cfg(feature = "doc")]
            deprecated: data.deprecated,
            #[cfg(feature = "doc")]
            args: data.args,
            #[cfg(feature = "doc")]
            return_type: data.return_type,
            #[cfg(feature = "doc")]
            argument_types: data.argument_types,
            docs,
        });

        let last = self.functions.last_mut().unwrap();

        Ok(ItemFnMut {
            docs: &mut last.docs,
            #[cfg(feature = "doc")]
            is_async: &mut last.is_async,
            #[cfg(feature = "doc")]
            deprecated: &mut last.deprecated,
            #[cfg(feature = "doc")]
            args: &mut last.args,
            #[cfg(feature = "doc")]
            return_type: &mut last.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut last.argument_types,
        })
    }

    /// Install an associated function.
    fn assoc_fn(
        &mut self,
        data: AssociatedFunctionData,
        docs: Docs,
    ) -> Result<ItemFnMut<'_>, ContextError> {
        if !self.names.insert(Name::Associated(data.assoc_key())) {
            return Err(match data.name.kind {
                meta::AssociatedKind::Protocol(protocol) => {
                    ContextError::ConflictingProtocolFunction {
                        type_info: data.container_type_info,
                        name: protocol.name.into(),
                    }
                }
                meta::AssociatedKind::FieldFn(protocol, field) => {
                    ContextError::ConflictingFieldFunction {
                        type_info: data.container_type_info,
                        name: protocol.name.into(),
                        field: field.into(),
                    }
                }
                meta::AssociatedKind::IndexFn(protocol, index) => {
                    ContextError::ConflictingIndexFunction {
                        type_info: data.container_type_info,
                        name: protocol.name.into(),
                        index,
                    }
                }
                meta::AssociatedKind::Instance(name) => ContextError::ConflictingInstanceFunction {
                    type_info: data.container_type_info,
                    name: name.into(),
                },
            });
        }

        self.associated.push(ModuleAssociated {
            container: data.container,
            container_type_info: data.container_type_info,
            name: data.name,
            handler: data.handler,
            #[cfg(feature = "doc")]
            is_async: data.is_async,
            #[cfg(feature = "doc")]
            deprecated: data.deprecated,
            #[cfg(feature = "doc")]
            args: data.args,
            #[cfg(feature = "doc")]
            return_type: data.return_type,
            #[cfg(feature = "doc")]
            argument_types: data.argument_types,
            docs,
        });

        let last = self.associated.last_mut().unwrap();

        Ok(ItemFnMut {
            docs: &mut last.docs,
            #[cfg(feature = "doc")]
            is_async: &mut last.is_async,
            #[cfg(feature = "doc")]
            deprecated: &mut last.deprecated,
            #[cfg(feature = "doc")]
            args: &mut last.args,
            #[cfg(feature = "doc")]
            return_type: &mut last.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut last.argument_types,
        })
    }
}

impl AsRef<Module> for Module {
    #[inline]
    fn as_ref(&self) -> &Module {
        self
    }
}
