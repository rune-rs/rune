//! Crate used for definint native *modules*.
//!
//! A native module is one that provides runestick functions and types
//! through native code.

use crate::collections::HashMap;
use crate::{
    Component, Future, Hash, Named, Stack, ToValue, Type, TypeInfo, TypeOf, UnsafeFromValue,
    VmError, VmErrorKind,
};
use std::any;
use std::any::type_name;
use std::future;
use std::sync::Arc;

use crate::context::{ContextError, Handler, Macro};
use crate::{GeneratorState, Item, StaticType, TypeCheck, Value};

/// Specialized information on `Option` types.
pub(crate) struct ModuleUnitType {
    /// Item of the unit type.
    pub(crate) item: Item,
}

/// Specialized information on `GeneratorState` types.
pub(crate) struct ModuleInternalEnum {
    /// The name of the internal enum.
    pub(crate) name: &'static str,
    /// The result type.
    pub(crate) base_type: Item,
    /// The static type of the enum.
    pub(crate) static_type: &'static StaticType,
    /// Internal variants.
    pub(crate) variants: Vec<ModuleInternalVariant>,
}

impl ModuleInternalEnum {
    /// Construct a new handler for an internal enum.
    pub fn new<N>(name: &'static str, base_type: N, static_type: &'static StaticType) -> Self
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        ModuleInternalEnum {
            name,
            base_type: Item::of(base_type),
            static_type,
            variants: Vec::new(),
        }
    }

    /// Register a new variant.
    fn variant<C, Args>(&mut self, name: &'static str, type_check: TypeCheck, constructor: C)
    where
        C: crate::module::Function<Args>,
        C::Return: TypeOf,
    {
        let constructor: Arc<Handler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));
        let type_of = C::Return::type_of();

        self.variants.push(ModuleInternalVariant {
            name,
            type_check,
            args: C::args(),
            constructor,
            type_of,
        });
    }
}

/// Internal variant.
pub(crate) struct ModuleInternalVariant {
    /// The name of the variant.
    pub(crate) name: &'static str,
    /// Type check for the variant.
    pub(crate) type_check: TypeCheck,
    /// Arguments for the variant.
    pub(crate) args: usize,
    /// The constructor of the variant.
    pub(crate) constructor: Arc<Handler>,
    /// The value type of the variant.
    pub(crate) type_of: Type,
}

pub(crate) struct ModuleType {
    /// The item of the installed type.
    pub(crate) name: Item,
    /// Type information for the installed type.
    pub(crate) type_info: TypeInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ModuleAssociatedKind {
    Getter,
    Instance,
}

impl ModuleAssociatedKind {
    /// Convert the kind into a hash function.
    pub fn into_hash_fn(self) -> fn(Type, Hash) -> Hash {
        match self {
            Self::Getter => Hash::getter,
            Self::Instance => Hash::instance_function,
        }
    }
}

pub(crate) struct ModuleAssociatedFn {
    pub(crate) handler: Arc<Handler>,
    pub(crate) args: Option<usize>,
    pub(crate) type_info: TypeInfo,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ModuleAssocKey {
    pub(crate) type_of: Type,
    pub(crate) hash: Hash,
    pub(crate) kind: ModuleAssociatedKind,
}

pub(crate) struct ModuleFn {
    pub(crate) handler: Arc<Handler>,
    pub(crate) args: Option<usize>,
}

pub(crate) struct ModuleMacro {
    pub(crate) handler: Arc<Macro>,
}

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct Module {
    /// The name of the module.
    pub(crate) path: Item,
    /// Free functions.
    pub(crate) functions: HashMap<Item, ModuleFn>,
    /// Macro handlers.
    pub(crate) macros: HashMap<Item, ModuleMacro>,
    /// Instance functions.
    pub(crate) associated_functions: HashMap<ModuleAssocKey, ModuleAssociatedFn>,
    /// Registered types.
    pub(crate) types: HashMap<Type, ModuleType>,
    /// Registered unit type.
    pub(crate) unit_type: Option<ModuleUnitType>,
    /// Registered generator state type.
    pub(crate) internal_enums: Vec<ModuleInternalEnum>,
}

impl Module {
    /// Construct a new module.
    pub fn new<I>(path: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        Self {
            path: Item::of(path),
            functions: Default::default(),
            macros: Default::default(),
            associated_functions: Default::default(),
            types: Default::default(),
            unit_type: None,
            internal_enums: Vec::new(),
        }
    }

    /// Create an empty module for the root path.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Register a type. Registering a type is mandatory in order to register
    /// instance functions using that type.
    ///
    /// This will allow the type to be used within scripts, using the item named
    /// here.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Any;
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
    /// # fn main() -> runestick::Result<()> {
    /// // Register `len` without registering a type.
    /// let mut module = runestick::Module::default();
    /// // Note: cannot do this until we have registered a type.
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = runestick::Context::new();
    /// assert!(context.install(&module).is_err());
    ///
    /// // Register `len` properly.
    /// let mut module = runestick::Module::default();
    ///
    /// module.ty::<MyBytes>()?;
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = runestick::Context::new();
    /// assert!(context.install(&module).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn ty<T>(&mut self) -> Result<(), ContextError>
    where
        T: Named + TypeOf,
    {
        let name = Item::of(&[T::NAME]);
        let type_of = T::type_of();
        let type_info = T::type_info();

        let ty = ModuleType {
            name: name.clone(),
            type_info,
        };

        if let Some(old) = self.types.insert(type_of, ty) {
            return Err(ContextError::ConflictingType {
                name,
                existing: old.type_info,
            });
        }

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
    /// ```rust
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::new(&["nonstd"]);
    /// module.unit(&["unit"])?;
    /// # Ok(())
    /// # }
    pub fn unit<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        if self.unit_type.is_some() {
            return Err(ContextError::UnitAlreadyPresent);
        }

        let item = Item::of(name);
        self.unit_type = Some(ModuleUnitType { item });
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
    /// ```rust
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::new(&["nonstd", "option"]);
    /// module.result(&["Option"])?;
    /// # Ok(())
    /// # }
    pub fn option<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ = ModuleInternalEnum::new("Option", name, crate::OPTION_TYPE);

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
    /// ```rust
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::new(&["nonstd", "result"]);
    /// module.result(&["Result"])?;
    /// # Ok(())
    /// # }
    pub fn result<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ = ModuleInternalEnum::new("Result", name, crate::RESULT_TYPE);

        // Note: these numeric variants are magic, and must simply match up with
        // what's being used in the virtual machine implementation for these
        // types.
        enum_.variant("Ok", TypeCheck::Result(0), Result::<Value, Value>::Ok);
        enum_.variant("Err", TypeCheck::Result(1), Result::<Value, Value>::Err);
        self.internal_enums.push(enum_);
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
    /// ```rust
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::new(&["nonstd", "generator"]);
    /// module.generator_state(&["GeneratorState"])?;
    /// # Ok(())
    /// # }
    pub fn generator_state<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ =
            ModuleInternalEnum::new("GeneratorState", name, crate::GENERATOR_STATE_TYPE);

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

    /// Register a function that cannot error internally.
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn add_ten(value: i64) -> i64 {
    ///     value + 10
    /// }
    ///
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.function(&["add_ten"], add_ten)?;
    /// module.function(&["empty"], || Ok::<_, runestick::Error>(()))?;
    /// module.function(&["string"], |a: String| Ok::<_, runestick::Error>(()))?;
    /// module.function(&["optional"], |a: Option<String>| Ok::<_, runestick::Error>(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: Function<Args>,
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let name = Item::of(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        self.functions.insert(
            name,
            ModuleFn {
                handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
                args: Some(Func::args()),
            },
        );

        Ok(())
    }

    /// Register a native macro handler.
    pub fn macro_<N, M, A, B, O>(&mut self, name: N, f: M) -> Result<(), ContextError>
    where
        M: 'static + Send + Sync + Copy + Fn(&mut A, &B) -> Result<O, crate::Error>,
        A: any::Any,
        B: any::Any,
        O: any::Any,
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let name = Item::of(name);

        if self.macros.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        let handler: Arc<Macro> = Arc::new(move |a, b| {
            let a = match a.downcast_mut::<A>() {
                Some(a) => a,
                None => {
                    return Err(crate::Error::msg(format!(
                        "expected argument #0 `{}`",
                        std::any::type_name::<A>()
                    )));
                }
            };

            let b = match b.downcast_ref::<B>() {
                Some(b) => b,
                None => {
                    return Err(crate::Error::msg(format!(
                        "expected argument #1 `{}`",
                        std::any::type_name::<B>()
                    )));
                }
            };

            let output = f(a, b)?;
            Ok(Box::new(output))
        });

        self.macros.insert(name, ModuleMacro { handler });
        Ok(())
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.async_function(&["empty"], || async { () })?;
    /// module.async_function(&["empty_fallible"], || async { Ok::<_, runestick::Error>(()) })?;
    /// module.async_function(&["string"], |a: String| async { Ok::<_, runestick::Error>(()) })?;
    /// module.async_function(&["optional"], |a: Option<String>| async { Ok::<_, runestick::Error>(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: AsyncFunction<Args>,
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let name = Item::of(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        self.functions.insert(
            name,
            ModuleFn {
                handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
                args: Some(Func::args()),
            },
        );

        Ok(())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<(), ContextError>
    where
        F: 'static + Copy + Fn(&mut Stack, usize) -> Result<(), VmError> + Send + Sync,
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let name = Item::of(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        self.functions.insert(
            name,
            ModuleFn {
                handler: Arc::new(move |stack, args| f(stack, args)),
                args: None,
            },
        );

        Ok(())
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Any;
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
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.ty::<MyBytes>()?;
    /// module.function(&["MyBytes", "new"], MyBytes::new)?;
    /// module.inst_fn("len", MyBytes::len)?;
    ///
    /// let mut context = runestick::Context::new();
    /// context.install(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: InstFn<Args>,
    {
        self.assoc_fn(name, f, ModuleAssociatedKind::Instance)
    }

    /// Install a getter for the specified field.
    pub fn getter<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: InstFn<Args>,
    {
        self.assoc_fn(name, f, ModuleAssociatedKind::Getter)
    }

    /// Install an associated function.
    fn assoc_fn<N, Func, Args>(
        &mut self,
        name: N,
        f: Func,
        kind: ModuleAssociatedKind,
    ) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: InstFn<Args>,
    {
        let type_of = Func::instance_type_of();
        let type_info = Func::instance_type_of_info();

        let key = ModuleAssocKey {
            type_of,
            hash: name.into_inst_fn_hash(),
            kind,
        };

        let name = name.into_name();

        if self.associated_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction { type_info, name });
        }

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));

        let instance_function = ModuleAssociatedFn {
            handler,
            args: Some(Func::args()),
            type_info,
            name,
        };

        self.associated_functions.insert(key, instance_function);
        Ok(())
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::atomic::AtomicU32;
    /// use std::sync::Arc;
    /// use runestick::Any;
    ///
    /// #[derive(Clone, Debug, Any)]
    /// struct MyType {
    ///     value: Arc<AtomicU32>,
    /// }
    ///
    /// impl MyType {
    ///     async fn test(&self) -> runestick::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.ty::<MyType>()?;
    /// module.async_inst_fn("test", MyType::test)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: AsyncInstFn<Args>,
    {
        let type_of = Func::instance_type_of();
        let type_info = Func::instance_type_of_info();

        let key = ModuleAssocKey {
            type_of,
            hash: name.into_inst_fn_hash(),
            kind: ModuleAssociatedKind::Instance,
        };

        let name = name.into_name();

        if self.associated_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction { type_info, name });
        }

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));

        let instance_function = ModuleAssociatedFn {
            handler,
            args: Some(Func::args()),
            type_info,
            name,
        };

        self.associated_functions.insert(key, instance_function);
        Ok(())
    }
}

/// Trait used to determine what can be used as an instance function name.
pub trait IntoInstFnHash: Copy {
    /// Generate a locally unique hash to check for conflicts.
    fn into_inst_fn_hash(self) -> Hash;

    /// Get a human readable name for the function.
    fn into_name(self) -> String;
}

impl<'a> IntoInstFnHash for &'a str {
    fn into_inst_fn_hash(self) -> Hash {
        Hash::of(self)
    }

    fn into_name(self) -> String {
        self.to_owned()
    }
}

/// Trait used to provide the [function][Module::function] function.
pub trait Function<Args>: 'static + Copy + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_function][Module::async_function] function.
pub trait AsyncFunction<Args>: 'static + Copy + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [inst_fn][Module::inst_fn] function.
pub trait InstFn<Args>: 'static + Copy + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_type_of() -> Type;

    /// Access the value type info of the instance.
    fn instance_type_of_info() -> TypeInfo;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_inst_fn][Module::async_inst_fn] function.
pub trait AsyncInstFn<Args>: 'static + Copy + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_type_of() -> Type;

    /// Access the value type of the instance.
    fn instance_type_of_info() -> TypeInfo;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
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
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Return,
            Return: ToValue,
            $($ty: UnsafeFromValue,)*
        {
            type Return = Return;

            fn args() -> usize {
                $count
            }

            fn fn_call(
                self,
                stack: &mut Stack,
                args: usize
            ) -> Result<(), VmError> {
                impl_register!{@check-args $count, args}

                #[allow(unused_mut)]
                let mut it = stack.drain_stack_top($count)?;
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

                    self($(<$ty>::to_arg($var.0),)*)
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }

        impl<Func, Return, $($ty,)*> AsyncFunction<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Return,
            Return: future::Future,
            Return::Output: ToValue,
            $($ty: 'static + UnsafeFromValue,)*
        {
            type Return = Return;

            fn args() -> usize {
                $count
            }

            fn fn_call(
                self,
                stack: &mut Stack,
                args: usize
            ) -> Result<(), VmError> {
                impl_register!{@check-args $count, args}

                #[allow(unused_mut)]
                let mut it = stack.drain_stack_top($count)?;
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@unsafe-vars $count, $($ty, $var, $num,)*}

                    Future::new(async move {
                        let output = self($(<$ty>::to_arg($var.0),)*).await;
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
            Func: 'static + Copy + Send + Sync + Fn(Instance $(, $ty)*) -> Return,
            Return: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn instance_type_of() -> Type {
                Instance::type_of()
            }

            fn instance_type_of_info() -> TypeInfo {
                Instance::type_info()
            }

            fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}

                #[allow(unused_mut)]
                let mut it = stack.drain_stack_top($count + 1)?;
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
                    self(Instance::to_arg(inst.0), $(<$ty>::to_arg($var.0),)*)
                };

                impl_register!{@return stack, ret, Return}
                Ok(())
            }
        }

        impl<Func, Return, Instance, $($ty,)*> AsyncInstFn<(Instance, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Instance $(, $ty)*) -> Return,
            Return: future::Future,
            Return::Output: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn instance_type_of() -> Type {
                Instance::type_of()
            }

            fn instance_type_of_info() -> TypeInfo {
                Instance::type_info()
            }

            fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}

                #[allow(unused_mut)]
                let mut it = stack.drain_stack_top($count + 1)?;
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

                    Future::new(async move {
                        let output = self(Instance::to_arg(inst.0), $(<$ty>::to_arg($var.0),)*).await;
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
            Err(e) => return Err(VmError::from(VmErrorKind::BadReturn {
                error: e.unpack_critical()?,
                ret: type_name::<$ty>()
            })),
        };

        $stack.push($ret);
    };

    // Expand to function variable bindings.
    (@unsafe-vars $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = match <$ty>::unsafe_from_value($var) {
                Ok(v) => v,
                Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                    error: e.unpack_critical()?,
                    arg: $count - $num,
                    to: type_name::<$ty>(),
                })),
            };
        )*
    };

    // Expand to instance variable bindings.
    (@unsafe-inst-vars $inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = match Instance::unsafe_from_value($inst) {
            Ok(v) => v,
            Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                error: e.unpack_critical()?,
                arg: 0,
                to: type_name::<Instance>()
            })),
        };

        $(
            let $var = match <$ty>::unsafe_from_value($var) {
                Ok(v) => v,
                Err(e) => return Err(VmError::from(VmErrorKind::BadArgument {
                    error: e.unpack_critical()?,
                    arg: 1 + $count - $num,
                    to: type_name::<$ty>()
                })),
            };
        )*
    };

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
