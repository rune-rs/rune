use crate::no_std::collections::{hash_map, HashMap};
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{self, ContextError, Docs, IntoComponent, ItemBuf, Named};
use crate::macros::{MacroContext, TokenStream};
use crate::module::function_meta::{
    AssociatedFunctionData, AssociatedFunctionKind, AssociatedFunctionName, FunctionData,
    FunctionMeta, FunctionMetaKind, IterFunctionArgs, MacroMeta, MacroMetaKind, ToFieldFunction,
    ToInstance,
};
use crate::module::{
    AssociatedFunctionKey, AsyncFunction, AsyncInstFn, Enum, Function, InstFn, InstallWith,
    InternalEnum, ModuleFunction, ModuleFunctionKind, ModuleMacro, Struct, Type, TypeSpecification,
    UnitType, Variant,
};
use crate::runtime::{
    ConstValue, FromValue, GeneratorState, MacroHandler, MaybeTypeOf, Protocol, Stack, ToValue,
    TypeCheck, TypeOf, Value, VmResult,
};
use crate::Hash;

/// A [Module] that is a collection of native functions and types.
///
/// Needs to be installed into a [Context][crate::compile::Context] using
/// [Context::install][crate::compile::Context::install].
#[derive(Default)]
pub struct Module {
    /// A special identifier for this module, which will cause it to not conflict if installed multiple times.
    pub(crate) unique: Option<&'static str>,
    /// The name of the module.
    pub(crate) item: ItemBuf,
    /// Functions.
    pub(crate) functions: HashMap<ItemBuf, ModuleFunction>,
    /// MacroHandler handlers.
    pub(crate) macros: HashMap<ItemBuf, ModuleMacro>,
    /// Constant values.
    pub(crate) constants: HashMap<ItemBuf, ConstValue>,
    /// Associated functions.
    pub(crate) associated_functions: HashMap<AssociatedFunctionKey, ModuleFunction>,
    /// Registered types.
    pub(crate) types: HashMap<Hash, Type>,
    /// Registered unit type.
    pub(crate) unit_type: Option<UnitType>,
    /// Registered generator state type.
    pub(crate) internal_enums: Vec<InternalEnum>,
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

    fn inner_new(item: ItemBuf) -> Self {
        Self {
            unique: None,
            item,
            functions: HashMap::default(),
            macros: HashMap::default(),
            associated_functions: HashMap::default(),
            types: HashMap::default(),
            unit_type: None,
            internal_enums: Vec::new(),
            constants: HashMap::default(),
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
    /// use rune::Any;
    ///
    /// #[derive(Any)]
    /// struct MyBytes {
    ///     queue: Vec<String>,
    /// }
    ///
    /// impl MyBytes {
    ///     fn len(&self) -> usize {
    ///         self.queue.len()
    ///     }
    /// }
    ///
    /// // Register `len` without registering a type.
    /// let mut module = rune::Module::default();
    /// // Note: cannot do this until we have registered a type.
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = rune::Context::new();
    /// assert!(context.install(module).is_err());
    ///
    /// // Register `len` properly.
    /// let mut module = rune::Module::default();
    ///
    /// module.ty::<MyBytes>()?;
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = rune::Context::new();
    /// assert!(context.install(module).is_ok());
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn ty<T>(&mut self) -> Result<(), ContextError>
    where
        T: Named + TypeOf + InstallWith,
    {
        let type_hash = T::type_hash();
        let type_info = T::type_info();

        let ty = Type {
            name: T::full_name(),
            type_info,
            spec: None,
        };

        if let Some(old) = self.types.insert(type_hash, ty) {
            return Err(ContextError::ConflictingType {
                item: ItemBuf::with_item(&[T::full_name()]),
                type_info: old.type_info,
            });
        }

        T::install_with(self)?;
        Ok(())
    }

    /// Register that the given type is a struct, and that it has the given
    /// compile-time metadata. This implies that each field has a
    /// [Protocol::GET] field function.
    ///
    /// This is typically not used directly, but is used automatically with the
    /// [Any][crate::Any] derive.
    pub fn struct_meta<T, const N: usize>(
        &mut self,
        fields: [&'static str; N],
    ) -> Result<(), ContextError>
    where
        T: Named + TypeOf,
    {
        let type_hash = T::type_hash();

        let ty = match self.types.get_mut(&type_hash) {
            Some(ty) => ty,
            None => {
                return Err(ContextError::MissingType {
                    item: ItemBuf::with_item(&[T::full_name()]),
                    type_info: T::type_info(),
                });
            }
        };

        let old = ty.spec.replace(TypeSpecification::Struct(Struct {
            fields: fields.into_iter().map(Box::<str>::from).collect(),
        }));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: ItemBuf::with_item(&[T::full_name()]),
                type_info: ty.type_info.clone(),
            });
        }

        Ok(())
    }

    /// Register enum metadata for the given type `T`. This allows an enum to be
    /// used in limited ways in Rune.
    pub fn enum_meta<T, const N: usize>(
        &mut self,
        variants: [(&'static str, Variant); N],
    ) -> Result<(), ContextError>
    where
        T: Named + TypeOf,
    {
        let type_hash = T::type_hash();

        let ty = match self.types.get_mut(&type_hash) {
            Some(ty) => ty,
            None => {
                return Err(ContextError::MissingType {
                    item: ItemBuf::with_item(&[T::full_name()]),
                    type_info: T::type_info(),
                });
            }
        };

        let old = ty.spec.replace(TypeSpecification::Enum(Enum {
            variants: variants
                .into_iter()
                .map(|(name, variant)| (Box::from(name), variant))
                .collect(),
        }));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: ItemBuf::with_item(&[T::full_name()]),
                type_info: ty.type_info.clone(),
            });
        }

        Ok(())
    }

    /// Register a variant constructor for type `T`.
    pub fn variant_constructor<Func, Args, T>(
        &mut self,
        index: usize,
        constructor: Func,
    ) -> Result<(), ContextError>
    where
        T: Named + TypeOf,
        Func: Function<Args, Return = T>,
    {
        let type_hash = T::type_hash();

        let ty = match self.types.get_mut(&type_hash) {
            Some(ty) => ty,
            None => {
                return Err(ContextError::MissingType {
                    item: ItemBuf::with_item(&[T::full_name()]),
                    type_info: T::type_info(),
                });
            }
        };

        let en = match &mut ty.spec {
            Some(TypeSpecification::Enum(en)) => en,
            _ => {
                return Err(ContextError::MissingEnum {
                    item: ItemBuf::with_item(&[T::full_name()]),
                    type_info: T::type_info(),
                });
            }
        };

        let variant = match en.variants.get_mut(index) {
            Some((_, variant)) => variant,
            _ => {
                return Err(ContextError::MissingVariant {
                    type_info: T::type_info(),
                    index,
                });
            }
        };

        if variant.constructor.is_some() {
            return Err(ContextError::VariantConstructorConflict {
                type_info: T::type_info(),
                index,
            });
        }

        variant.constructor = Some(Arc::new(move |stack, args| {
            constructor.fn_call(stack, args)
        }));

        Ok(())
    }

    /// Construct type information for the `unit` type.
    ///
    /// Registering this allows the given type to be used in Rune scripts when
    /// referring to the `unit` type.
    ///
    /// # Examples
    ///
    /// This shows how to register the unit type `()` as `nonstd::unit`.
    ///
    /// ```
    /// use rune::Module;
    ///
    /// let mut module = Module::with_item(["nonstd"]);
    /// module.unit("unit")?;
    /// # Ok::<_, rune::Error>(())
    pub fn unit<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: AsRef<str>,
    {
        if self.unit_type.is_some() {
            return Err(ContextError::UnitAlreadyPresent);
        }

        self.unit_type = Some(UnitType {
            name: <Box<str>>::from(name.as_ref()),
        });

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
    pub fn generator_state<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ =
            InternalEnum::new("GeneratorState", name, crate::runtime::GENERATOR_STATE_TYPE);

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
        Ok(())
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
    pub fn option<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Option", name, crate::runtime::OPTION_TYPE);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Some", TypeCheck::Option(0), Option::<Value>::Some);
        enum_.variant("None", TypeCheck::Option(1), || Option::<Value>::None);
        self.internal_enums.push(enum_);
        Ok(())
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
    pub fn result<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let mut enum_ = InternalEnum::new("Result", name, crate::runtime::RESULT_TYPE);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Ok", TypeCheck::Result(0), Result::<Value, Value>::Ok);
        enum_.variant("Err", TypeCheck::Result(1), Result::<Value, Value>::Err);
        self.internal_enums.push(enum_);
        Ok(())
    }

    /// Register a constant value, at a crate, module or associated level.
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// let mut module = rune::Module::default();
    ///
    /// module.constant(["TEN"], 10)?; // a global TEN value
    /// module.constant(["MyType", "TEN"], 10)?; // looks like an associated value
    ///
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn constant<N, V>(&mut self, name: N, value: V) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        V: ToValue,
    {
        let name = ItemBuf::with_item(name);

        if self.constants.contains_key(&name) {
            return Err(ContextError::ConflictingConstantName { name });
        }

        let value = match value.to_value() {
            VmResult::Ok(v) => v,
            VmResult::Err(error) => return Err(ContextError::ValueError { error }),
        };

        let constant_value = match <ConstValue as FromValue>::from_value(value) {
            VmResult::Ok(v) => v,
            VmResult::Err(error) => return Err(ContextError::ValueError { error }),
        };

        self.constants.insert(name, constant_value);
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
    ///
    /// /// Takes an identifier and converts it into a string.
    /// ///
    /// /// # Examples
    /// ///
    /// /// ```rune
    /// /// assert_eq!(ident_to_string!(Hello), "Hello");
    /// /// ```
    /// #[rune::macro_]
    /// fn ident_to_string(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    ///     let ident = p.parse_all::<ast::Ident>()?;
    ///     let ident = ctx.resolve(ident)?.to_owned();
    ///     let string = ctx.lit(&ident);
    ///     Ok(quote!(#string).into_token_stream(ctx))
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_meta(ident_to_string)?;
    /// Ok::<_, rune::Error>(())
    /// ```
    #[inline]
    pub fn macro_meta(&mut self, meta: MacroMeta) -> Result<&mut ModuleMacro, ContextError> {
        let meta = meta();

        match meta.kind {
            MacroMetaKind::Function(data) => match self.macros.entry(data.name.clone()) {
                hash_map::Entry::Occupied(..) => {
                    Err(ContextError::ConflictingMacroName { name: data.name })
                }
                hash_map::Entry::Vacant(e) => {
                    let mut docs = Docs::default();
                    docs.set_docs(meta.docs);

                    Ok(e.insert(ModuleMacro {
                        handler: data.handler,
                        docs,
                    }))
                }
            },
        }
    }

    /// Register a native macro handler.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes more
    /// useful information about the function.
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
    /// fn ident_to_string(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> compile::Result<TokenStream> {
    ///     let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    ///     let ident = p.parse_all::<ast::Ident>()?;
    ///     let ident = ctx.resolve(ident)?.to_owned();
    ///     let string = ctx.lit(&ident);
    ///     Ok(quote!(#string).into_token_stream(ctx))
    /// }
    ///
    /// let mut m = Module::new();
    /// m.macro_(["ident_to_string"], ident_to_string)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn macro_<N, M>(&mut self, name: N, f: M) -> Result<&mut ModuleMacro, ContextError>
    where
        M: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let name = ItemBuf::with_item(name);

        match self.macros.entry(name.clone()) {
            hash_map::Entry::Occupied(..) => Err(ContextError::ConflictingMacroName { name }),
            hash_map::Entry::Vacant(e) => {
                let handler: Arc<MacroHandler> = Arc::new(f);

                Ok(e.insert(ModuleMacro {
                    handler,
                    docs: Docs::default(),
                }))
            }
        }
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
    ///
    /// /// This is a pretty neat function.
    /// #[rune::function]
    /// fn to_string(string: &str) -> String {
    ///     string.to_string()
    /// }
    ///
    /// /// This is a pretty neat download function
    /// #[rune::function]
    /// async fn download(url: &str) -> rune::Result<String> {
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
    /// use rune::Any;
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
    ///     #[rune::function]
    ///     async fn download(&self, url: &str) -> rune::Result<()> {
    ///         todo!()
    ///     }
    /// }
    ///
    /// let mut module = rune::Module::default();
    ///
    /// module.ty::<MyBytes>()?;
    /// module.function_meta(MyBytes::len)?;
    /// module.function_meta(MyBytes::download)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    #[inline]
    pub fn function_meta(
        &mut self,
        meta: FunctionMeta,
    ) -> Result<&mut ModuleFunction, ContextError> {
        let meta = meta();

        match meta.kind {
            FunctionMetaKind::Function(data) => {
                let mut docs = Docs::default();
                docs.set_docs(meta.docs);
                docs.set_arguments(meta.arguments);
                self.function_inner(data, docs)
            }
            FunctionMetaKind::AssociatedFunction(data) => {
                let mut docs = Docs::default();
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
    /// fn add_ten(value: i64) -> i64 {
    ///     value + 10
    /// }
    ///
    /// let mut module = rune::Module::default();
    ///
    /// module.function(["add_ten"], add_ten)?;
    /// module.function(["empty"], || Ok::<_, rune::Error>(()))?;
    /// module.function(["string"], |a: String| Ok::<_, rune::Error>(()))?;
    /// module.function(["optional"], |a: Option<String>| Ok::<_, rune::Error>(()))?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn function<Func, Args, N>(
        &mut self,
        name: N,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        Func: Function<Args>,
        Func::Return: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        Args: IterFunctionArgs,
    {
        self.function_inner(FunctionData::new(name, f), Docs::default())
    }

    /// Register an asynchronous function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes
    /// more useful information about the function.
    ///
    /// This returns a mutable [`ModuleFunction`], which can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut module = rune::Module::default();
    ///
    /// async fn empty() {
    /// }
    ///
    /// async fn empty_fallible() -> rune::Result<()> {
    ///     Ok(())
    /// }
    ///
    /// async fn string(a: String) -> rune::Result<()> {
    ///     Ok(())
    /// }
    ///
    /// async fn optional(a: Option<String>) -> rune::Result<()> {
    ///     Ok(())
    /// }
    ///
    /// module.async_function(["empty"], empty)?;
    /// module.async_function(["empty_fallible"], empty_fallible)?;
    /// module.async_function(["string"], string)?;
    /// module.async_function(["optional"], optional)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn async_function<Func, Args, N>(
        &mut self,
        name: N,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        Func: AsyncFunction<Args>,
        Func::Output: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        Args: IterFunctionArgs,
    {
        self.function_inner(FunctionData::new_async(name, f), Docs::default())
    }

    /// Register an instance function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes
    /// more useful information about the function.
    ///
    /// This returns a mutable [`ModuleFunction`], which can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
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
    ///     fn len(&self) -> usize {
    ///         self.queue.len()
    ///     }
    /// }
    ///
    /// let mut module = rune::Module::default();
    ///
    /// module.ty::<MyBytes>()?;
    /// module.function(["MyBytes", "new"], MyBytes::new)?;
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = rune::Context::new();
    /// context.install(module)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn inst_fn<N, Func, Args>(
        &mut self,
        name: N,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        N: ToInstance,
        Func: InstFn<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        self.assoc_fn(
            AssociatedFunctionData::new(name.to_instance(), f),
            Docs::default(),
        )
    }

    /// Register an asynchronous instance function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes
    /// more useful information about the function.
    ///
    /// This returns a mutable [`ModuleFunction`], which can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomic::AtomicU32;
    /// use std::sync::Arc;
    /// use rune::Any;
    ///
    /// #[derive(Clone, Debug, Any)]
    /// struct MyType {
    ///     value: Arc<AtomicU32>,
    /// }
    ///
    /// impl MyType {
    ///     async fn test(&self) -> rune::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut module = rune::Module::default();
    ///
    /// module.ty::<MyType>()?;
    /// module.async_inst_fn("test", MyType::test)?;
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn async_inst_fn<N, Func, Args>(
        &mut self,
        name: N,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        N: ToInstance,
        Func: AsyncInstFn<Args>,
        Func::Output: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        self.assoc_fn(
            AssociatedFunctionData::new_async(name.to_instance(), f),
            Docs::default(),
        )
    }

    /// Install a protocol function that interacts with the given field.
    ///
    /// This returns a mutable [`ModuleFunction`], which can be used to
    /// associate more metadata with the inserted item.
    pub fn field_fn<N, Func, Args>(
        &mut self,
        protocol: Protocol,
        name: N,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        N: ToFieldFunction,
        Func: InstFn<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        self.assoc_fn(
            AssociatedFunctionData::new(name.to_field_function(protocol), f),
            Docs::default(),
        )
    }

    /// Install a protocol function that interacts with the given index.
    ///
    /// An index can either be a field inside a tuple, or a variant inside of an
    /// enum as configured with [Module::enum_meta].
    pub fn index_fn<Func, Args>(
        &mut self,
        protocol: Protocol,
        index: usize,
        f: Func,
    ) -> Result<&mut ModuleFunction, ContextError>
    where
        Func: InstFn<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        let name = AssociatedFunctionName::index(protocol, index);
        self.assoc_fn(AssociatedFunctionData::new(name, f), Docs::default())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    ///
    /// This returns a mutable [`ModuleFunction`], which can be used to
    /// associate more metadata with the inserted item.
    ///
    /// # Examples
    ///
    /// ```
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
    /// let mut module = rune::Module::default();
    ///
    /// let sum = module.raw_fn(["sum"], sum)?;
    /// sum.docs([
    ///     "Sum all numbers provided to the function."
    /// ]);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<&mut ModuleFunction, ContextError>
    where
        F: 'static + Fn(&mut Stack, usize) -> VmResult<()> + Send + Sync,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let name = ItemBuf::with_item(name);

        if self.functions.contains_key(&name) {}

        match self.functions.entry(name) {
            hash_map::Entry::Occupied(e) => Err(ContextError::ConflictingFunctionName {
                name: e.key().clone(),
            }),
            hash_map::Entry::Vacant(e) => Ok(e.insert(ModuleFunction {
                handler: Arc::new(move |stack, args| f(stack, args)),
                is_async: false,
                args: None,
                return_type: None,
                argument_types: Box::from([]),
                docs: Docs::default(),
                kind: ModuleFunctionKind::Function,
            })),
        }
    }

    fn function_inner(
        &mut self,
        data: FunctionData,
        docs: Docs,
    ) -> Result<&mut ModuleFunction, ContextError> {
        match self.functions.entry(data.name.clone()) {
            hash_map::Entry::Occupied(e) => Err(ContextError::ConflictingFunctionName {
                name: e.key().clone(),
            }),
            hash_map::Entry::Vacant(e) => Ok(e.insert(ModuleFunction {
                handler: data.handler,
                is_async: data.is_async,
                args: data.args,
                return_type: data.return_type,
                argument_types: data.argument_types,
                docs,
                kind: ModuleFunctionKind::Function,
            })),
        }
    }

    /// Install an associated function.
    fn assoc_fn(
        &mut self,
        data: AssociatedFunctionData,
        docs: Docs,
    ) -> Result<&mut ModuleFunction, ContextError> {
        let key = data.assoc_key();

        match self.associated_functions.entry(key) {
            hash_map::Entry::Occupied(..) => Err(match data.name.kind {
                AssociatedFunctionKind::Protocol(protocol) => {
                    ContextError::ConflictingProtocolFunction {
                        type_info: data.ty.type_info,
                        name: protocol.name.into(),
                    }
                }
                AssociatedFunctionKind::FieldFn(protocol, field) => {
                    ContextError::ConflictingFieldFunction {
                        type_info: data.ty.type_info,
                        name: protocol.name.into(),
                        field,
                    }
                }
                AssociatedFunctionKind::IndexFn(protocol, index) => {
                    ContextError::ConflictingIndexFunction {
                        type_info: data.ty.type_info,
                        name: protocol.name.into(),
                        index,
                    }
                }
                AssociatedFunctionKind::Instance(name) => {
                    ContextError::ConflictingInstanceFunction {
                        type_info: data.ty.type_info,
                        name,
                    }
                }
            }),
            hash_map::Entry::Vacant(e) => Ok(e.insert(ModuleFunction {
                handler: data.handler,
                is_async: data.is_async,
                args: data.args,
                return_type: data.return_type,
                argument_types: data.argument_types,
                docs,
                kind: ModuleFunctionKind::Assoc {
                    type_info: data.ty.type_info,
                    name: data.name,
                },
            })),
        }
    }
}

impl AsRef<Module> for Module {
    #[inline]
    fn as_ref(&self) -> &Module {
        self
    }
}
