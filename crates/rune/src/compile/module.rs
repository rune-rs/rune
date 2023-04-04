//! Crate used for definint native *modules*.
//!
//! A native module is one that provides rune with functions and types through
//! native code.

use std::fmt;
use std::future;
use std::sync::Arc;

use crate::collections::{HashMap, HashSet};
use crate::compile::{
    AssociatedFunctionData, AssociatedFunctionKind, AssociatedFunctionName, ContextError, Docs,
    FunctionData, FunctionMeta, FunctionMetaKind, IntoComponent, ItemBuf, Named, ToFieldFunction,
    ToInstance,
};
use crate::macros::{MacroContext, TokenStream};
use crate::runtime::{
    ConstValue, FromValue, FunctionHandler, Future, GeneratorState, MacroHandler, Protocol, Stack,
    StaticType, ToValue, TypeCheck, TypeInfo, TypeOf, UnsafeFromValue, Value, VmError, VmErrorKind,
};
use crate::Hash;

/// Trait to handle the installation of auxilliary functions for a type
/// installed into a module.
pub trait InstallWith {
    /// Hook to install more things into the module.
    fn install_with(_: &mut Module) -> Result<(), ContextError> {
        Ok(())
    }
}

/// The static hash and diagnostical information about a type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AssocType {
    /// Hash of the type.
    pub hash: Hash,
    /// Type information of the instance function.
    pub type_info: TypeInfo,
}

/// Specialized information on `Option` types.
pub(crate) struct UnitType {
    /// Item of the unit type.
    pub(crate) name: Box<str>,
}

/// Specialized information on `GeneratorState` types.
pub(crate) struct InternalEnum {
    /// The name of the internal enum.
    pub(crate) name: &'static str,
    /// The result type.
    pub(crate) base_type: ItemBuf,
    /// The static type of the enum.
    pub(crate) static_type: &'static StaticType,
    /// Internal variants.
    pub(crate) variants: Vec<InternalVariant>,
}

impl InternalEnum {
    /// Construct a new handler for an internal enum.
    fn new<N>(name: &'static str, base_type: N, static_type: &'static StaticType) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        InternalEnum {
            name,
            base_type: ItemBuf::with_item(base_type),
            static_type,
            variants: Vec::new(),
        }
    }

    /// Register a new variant.
    fn variant<C, Args>(&mut self, name: &'static str, type_check: TypeCheck, constructor: C)
    where
        C: Function<Args>,
    {
        let constructor: Arc<FunctionHandler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.variants.push(InternalVariant {
            name,
            type_check,
            args: C::args(),
            constructor,
        });
    }
}

/// Internal variant.
pub(crate) struct InternalVariant {
    /// The name of the variant.
    pub(crate) name: &'static str,
    /// Type check for the variant.
    pub(crate) type_check: TypeCheck,
    /// Arguments for the variant.
    pub(crate) args: usize,
    /// The constructor of the variant.
    pub(crate) constructor: Arc<FunctionHandler>,
}

/// Data for an opaque type. If `spec` is set, indicates things which are known
/// about that type.
pub(crate) struct Type {
    /// The name of the installed type which will be the final component in the
    /// item it will constitute.
    pub(crate) name: Box<str>,
    /// Type information for the installed type.
    pub(crate) type_info: TypeInfo,
    /// The specification for the type.
    pub(crate) spec: Option<TypeSpecification>,
}

/// Metadata about a variant.
pub struct Variant {
    /// Variant metadata.
    pub(crate) kind: VariantKind,
    /// Handler to use if this variant can be constructed through a regular function call.
    pub(crate) constructor: Option<Arc<FunctionHandler>>,
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variant")
            .field("kind", &self.kind)
            .field("constructor", &self.constructor.is_some())
            .finish()
    }
}

/// The kind of the variant.
#[derive(Debug)]
pub(crate) enum VariantKind {
    /// Variant is a Tuple variant.
    Tuple(Tuple),
    /// Variant is a Struct variant.
    Struct(Struct),
    /// Variant is a Unit variant.
    Unit,
}

impl Variant {
    /// Construct metadata for a tuple variant.
    #[inline]
    pub fn tuple(args: usize) -> Self {
        Self {
            kind: VariantKind::Tuple(Tuple { args }),
            constructor: None,
        }
    }

    /// Construct metadata for a tuple variant.
    #[inline]
    pub fn st<const N: usize>(fields: [&'static str; N]) -> Self {
        Self {
            kind: VariantKind::Struct(Struct {
                fields: fields.into_iter().map(Box::<str>::from).collect(),
            }),
            constructor: None,
        }
    }

    /// Construct metadata for a unit variant.
    #[inline]
    pub fn unit() -> Self {
        Self {
            kind: VariantKind::Unit,
            constructor: None,
        }
    }
}

/// Metadata about a tuple or tuple variant.
#[derive(Debug)]
pub struct Tuple {
    /// The number of fields.
    pub(crate) args: usize,
}

/// The type specification for a native struct.
#[derive(Debug)]
pub(crate) struct Struct {
    /// The names of the struct fields known at compile time.
    pub(crate) fields: HashSet<Box<str>>,
}

/// The type specification for a native enum.
pub(crate) struct Enum {
    /// The variants.
    pub(crate) variants: Vec<(Box<str>, Variant)>,
}

/// A type specification.
pub(crate) enum TypeSpecification {
    Struct(Struct),
    Enum(Enum),
}

/// The data of an associated function.
#[derive(Clone)]
#[non_exhaustive]
pub struct AssociatedFunction {
    /// Handle of the associated function.
    pub(crate) handler: Arc<FunctionHandler>,
    /// Type information of the associated function.
    pub(crate) type_info: TypeInfo,
    /// If the function is asynchronous.
    pub(crate) is_async: bool,
    /// Arguments the function receives.
    pub(crate) args: Option<usize>,
    /// The full name of the associated function.
    pub(crate) name: AssociatedFunctionName,
    /// The documentation of the associated function.
    pub(crate) docs: Docs,
}

/// A key that identifies an associated function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AssociatedFunctionKey {
    /// The type the associated function belongs to.
    pub type_hash: Hash,
    /// The kind of the associated function.
    pub kind: AssociatedFunctionKind,
    /// The type parameters of the associated function.
    pub parameters: Hash,
}

pub(crate) struct ModuleFunction {
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) is_async: bool,
    pub(crate) args: Option<usize>,
    pub(crate) instance_function: bool,
    pub(crate) docs: Docs,
}

pub(crate) struct Macro {
    pub(crate) handler: Arc<MacroHandler>,
}

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
    pub(crate) macros: HashMap<ItemBuf, Macro>,
    /// Constant values.
    pub(crate) constants: HashMap<ItemBuf, ConstValue>,
    /// Associated functions.
    pub(crate) associated_functions: HashMap<AssociatedFunctionKey, AssociatedFunction>,
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
            Ok(v) => v,
            Err(e) => return Err(ContextError::ValueError { error: e }),
        };

        let constant_value = match <ConstValue as FromValue>::from_value(value) {
            Ok(v) => v,
            Err(e) => return Err(ContextError::ValueError { error: e }),
        };

        self.constants.insert(name, constant_value);

        Ok(())
    }

    /// Register a native macro handler.
    pub fn macro_<N, M>(&mut self, name: N, f: M) -> Result<(), ContextError>
    where
        M: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_>, &TokenStream) -> crate::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let name = ItemBuf::with_item(name);

        if self.macros.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        let handler: Arc<MacroHandler> = Arc::new(f);
        self.macros.insert(name, Macro { handler });
        Ok(())
    }

    /// Register a function from its meta information.
    ///
    /// The metadata must be provided by annotating the function with
    /// [`#[rune::function]`][crate::function].
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
    pub fn function_meta(&mut self, meta: FunctionMeta) -> Result<(), ContextError> {
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
    pub fn function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: Function<Args>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        self.function_inner(FunctionData::new(name, f), Docs::default())
    }

    /// Register an asynchronous function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes more
    /// useful information about the function.
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
    pub fn async_function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: AsyncFunction<Args>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        self.function_inner(FunctionData::new_async(name, f), Docs::default())
    }

    /// Register an instance function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes more
    /// useful information about the function.
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
    pub fn inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: ToInstance,
        Func: InstFn<Args>,
    {
        self.assoc_fn(
            AssociatedFunctionData::new(name.to_instance(), f),
            Docs::default(),
        )
    }

    /// Register an asynchronous instance function.
    ///
    /// If possible, [`Module::function_meta`] should be used since it includes more
    /// useful information about the function.
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
    pub fn async_inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: ToInstance,
        Func: AsyncInstFn<Args>,
    {
        self.assoc_fn(
            AssociatedFunctionData::new_async(name.to_instance(), f),
            Docs::default(),
        )
    }

    /// Install a protocol function that interacts with the given field.
    pub fn field_fn<N, Func, Args>(
        &mut self,
        protocol: Protocol,
        name: N,
        f: Func,
    ) -> Result<(), ContextError>
    where
        N: ToFieldFunction,
        Func: InstFn<Args>,
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
    ) -> Result<(), ContextError>
    where
        Func: InstFn<Args>,
    {
        let name = AssociatedFunctionName::index(protocol, index);

        self.assoc_fn(AssociatedFunctionData::new(name, f), Docs::default())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<(), ContextError>
    where
        F: 'static + Fn(&mut Stack, usize) -> Result<(), VmError> + Send + Sync,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        let name = ItemBuf::with_item(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        self.functions.insert(
            name,
            ModuleFunction {
                handler: Arc::new(move |stack, args| f(stack, args)),
                is_async: false,
                args: None,
                instance_function: false,
                docs: Docs::default(),
            },
        );

        Ok(())
    }

    fn function_inner(&mut self, data: FunctionData, docs: Docs) -> Result<(), ContextError> {
        if self.functions.contains_key(&data.name) {
            return Err(ContextError::ConflictingFunctionName { name: data.name });
        }

        self.functions.insert(
            data.name,
            ModuleFunction {
                handler: data.handler,
                is_async: data.is_async,
                args: data.args,
                instance_function: false,
                docs,
            },
        );

        Ok(())
    }

    /// Install an associated function.
    fn assoc_fn(&mut self, data: AssociatedFunctionData, docs: Docs) -> Result<(), ContextError> {
        let key = data.assoc_key();

        if self.associated_functions.contains_key(&key) {
            return Err(match data.name.kind {
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
            });
        }

        let assoc_fn = AssociatedFunction {
            handler: data.handler,
            type_info: data.ty.type_info,
            is_async: data.is_async,
            args: data.args,
            name: data.name,
            docs,
        };

        self.associated_functions.insert(key, assoc_fn);
        Ok(())
    }
}

impl AsRef<Module> for Module {
    #[inline]
    fn as_ref(&self) -> &Module {
        self
    }
}

/// Trait used to provide the [function][Module::function] function.
pub trait Function<Args>: 'static + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_function][Module::async_function] function.
pub trait AsyncFunction<Args>: 'static + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [inst_fn][Module::inst_fn] function.
pub trait InstFn<Args>: 'static + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    fn ty() -> AssocType;

    /// Perform the vm call.
    fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_inst_fn][Module::async_inst_fn] function.
pub trait AsyncInstFn<Args>: 'static + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    fn ty() -> AssocType;

    /// Perform the vm call.
    fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

macro_rules! impl_register {
    () => {
        impl_register!{@impl 0,}
    };

    ({$ty:ident, $var:ident, $num:expr}, $({$l_ty:ident, $l_var:ident, $l_num:expr},)*) => {
        impl_register!{@impl $num, {$ty, $var, $num}, $({$l_ty, $l_var, $l_num},)*}
        impl_register!{$({$l_ty, $l_var, $l_num},)*}
    };

    (@impl $count:expr, $({$ty:ident, $var:ident, $num:expr},)*) => {
        impl<Func, Return, $($ty,)*> Function<($($ty,)*)> for Func
        where
            Func: 'static + Send + Sync + Fn($($ty,)*) -> Return,
            Return: ToValue,
            $($ty: UnsafeFromValue,)*
        {
            type Return = Return;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args $count, args}

                #[allow(unused_mut)]
                let mut it = stack.drain($count)?;
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `stack`
                // when we return below.
                #[allow(unused)]
                let ret = unsafe {
                    impl_register!{@unsafe-vars $count, $($ty, $var, $num,)*}
                    let ret = self($(<$ty>::unsafe_coerce($var.0),)*);
                    impl_register!{@drop-stack-guards $($var),*}
                    ret
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }

        impl<Func, Return, $($ty,)*> AsyncFunction<($($ty,)*)> for Func
        where
            Func: 'static + Send + Sync + Fn($($ty,)*) -> Return,
            Return: 'static + future::Future,
            Return::Output: ToValue,
            $($ty: 'static + UnsafeFromValue,)*
        {
            type Return = Return;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args $count, args}

                #[allow(unused_mut)]
                let mut it = stack.drain($count)?;
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@unsafe-vars $count, $($ty, $var, $num,)*}

                    let fut = self($(<$ty>::unsafe_coerce($var.0),)*);

                    Future::new(async move {
                        let output = fut.await;
                        impl_register!{@drop-stack-guards $($var),*}
                        let value = output.to_value()?;
                        Ok(value)
                    })
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }

        impl<Func, Return, Instance, $($ty,)*> InstFn<(Instance, $($ty,)*)> for Func
        where
            Func: 'static + Send + Sync + Fn(Instance $(, $ty)*) -> Return,
            Return: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn ty() -> AssocType {
                AssocType {
                    hash: Instance::type_hash(),
                    type_info: Instance::type_info(),
                }
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}

                #[allow(unused_mut)]
                let mut it = stack.drain($count + 1)?;
                let inst = it.next().unwrap();
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `stack`
                // when we return below.
                #[allow(unused)]
                let ret = unsafe {
                    impl_register!{@unsafe-inst-vars inst, $count, $($ty, $var, $num,)*}
                    let ret = self(Instance::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);
                    impl_register!{@drop-stack-guards inst, $($var),*}
                    ret
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }

        impl<Func, Return, Instance, $($ty,)*> AsyncInstFn<(Instance, $($ty,)*)> for Func
        where
            Func: 'static + Send + Sync + Fn(Instance $(, $ty)*) -> Return,
            Return: 'static + future::Future,
            Return::Output: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn ty() -> AssocType {
                AssocType {
                    hash: Instance::type_hash(),
                    type_info: Instance::type_info(),
                }
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}

                #[allow(unused_mut)]
                let mut it = stack.drain($count + 1)?;
                let inst = it.next().unwrap();
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused)]
                let ret = unsafe {
                    impl_register!{@unsafe-inst-vars inst, $count, $($ty, $var, $num,)*}

                    let fut = self(Instance::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);

                    Future::new(async move {
                        let output = fut.await;
                        impl_register!{@drop-stack-guards inst, $($var),*}
                        let value = output.to_value()?;
                        Ok(value)
                    })
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }
    };

    (@return $stack:ident, $ret:ident, $ty:ty) => {
        let $ret = match $ret.to_value() {
            Ok($ret) => $ret,
            Err(e) => return Err(VmError::from(e.unpack_critical()?)),
        };

        $stack.push($ret);
    };

    // Expand to function variable bindings.
    (@unsafe-vars $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = match <$ty>::from_value($var) {
                Ok(v) => v,
                Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                    error: e.unpack_critical()?,
                    arg: $count - $num,
                })),
            };
        )*
    };

    // Expand to instance variable bindings.
    (@unsafe-inst-vars $inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = match Instance::from_value($inst) {
            Ok(v) => v,
            Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                error: e.unpack_critical()?,
                arg: 0,
            })),
        };

        $(
            let $var = match <$ty>::from_value($var) {
                Ok(v) => v,
                Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                    error: e.unpack_critical()?,
                    arg: 1 + $count - $num,
                })),
            };
        )*
    };

    // Helper variation to drop all stack guards associated with the specified variables.
    (@drop-stack-guards $($var:ident),* $(,)?) => {{
        $(drop(($var.1));)*
    }};

    (@check-args $expected:expr, $actual:expr) => {
        if $actual != $expected {
            return Err(VmError::from(VmErrorKind::BadArgumentCount {
                actual: $actual,
                expected: $expected,
            }));
        }
    };
}

repeat_macro!(impl_register);
