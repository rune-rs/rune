use crate::collections::HashMap;
use crate::{
    Component, Future, Hash, ReflectValueType, Stack, ToValue, UnsafeFromValue, ValueError,
    ValueType, ValueTypeInfo, VmError, VmErrorKind,
};
use std::any::type_name;
use std::future;
use std::sync::Arc;

use crate::context::{ContextError, Handler, IntoInstFnHash};
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
        C: crate::Function<Args>,
        C::Return: ReflectValueType,
    {
        let constructor: Arc<Handler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));
        let value_type = C::Return::value_type();

        self.variants.push(ModuleInternalVariant {
            name,
            type_check,
            args: C::args(),
            constructor,
            value_type,
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
    pub(crate) value_type: ValueType,
}

pub(crate) struct ModuleType {
    /// The item of the installed type.
    pub(crate) name: Item,
    /// Type information for the installed type.
    pub(crate) value_type_info: ValueTypeInfo,
}

pub(crate) struct ModuleInstanceFunction {
    pub(crate) handler: Arc<Handler>,
    pub(crate) args: Option<usize>,
    pub(crate) value_type_info: ValueTypeInfo,
    pub(crate) name: String,
}

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct Module {
    /// The name of the module.
    pub(crate) path: Item,
    /// Free functions.
    pub(crate) functions: HashMap<Item, (Arc<Handler>, Option<usize>)>,
    /// Instance functions.
    pub(crate) instance_functions: HashMap<(ValueType, Hash), ModuleInstanceFunction>,
    /// Registered types.
    pub(crate) types: HashMap<ValueType, ModuleType>,
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
            instance_functions: Default::default(),
            types: Default::default(),
            unit_type: None,
            internal_enums: Vec::new(),
        }
    }

    /// Register a type.
    ///
    /// This will allow the type to be used within scripts, using the item named
    /// here.
    pub fn ty<N>(&mut self, name: N) -> TypeBuilder<'_, N>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        TypeBuilder {
            name,
            types: &mut self.types,
        }
    }

    /// Construct the option type.
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

    /// Construct the option type.
    pub fn option<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ = ModuleInternalEnum::new("Option", name, crate::OPTION_TYPE);
        enum_.variant("Some", TypeCheck::Option(0), Option::<Value>::Some);
        enum_.variant("None", TypeCheck::Option(1), || Option::<Value>::None);
        self.internal_enums.push(enum_);
        Ok(())
    }

    /// Construct the result type.
    pub fn result<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ = ModuleInternalEnum::new("Result", name, crate::RESULT_TYPE);
        enum_.variant("Ok", TypeCheck::Result(0), Result::<Value, Value>::Ok);
        enum_.variant("Err", TypeCheck::Result(1), Result::<Value, Value>::Err);
        self.internal_enums.push(enum_);
        return Ok(());
    }

    /// Construct the type information for the `GeneratorState` type.
    pub fn generator_state<N>(&mut self, name: N) -> Result<(), ContextError>
    where
        N: IntoIterator,
        N::Item: Into<Component>,
    {
        let mut enum_ =
            ModuleInternalEnum::new("GeneratorState", name, crate::GENERATOR_STATE_TYPE);

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
    /// use std::collections::VecDeque;
    ///
    /// #[derive(Debug, Clone)]
    /// struct StringQueue(VecDeque<String>);
    ///
    /// impl StringQueue {
    ///     fn new() -> Self {
    ///         Self(VecDeque::new())
    ///     }
    /// }
    ///
    /// runestick::decl_external!(StringQueue);
    ///
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.function(&["bytes"], StringQueue::new)?;
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

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));
        self.functions.insert(name, (handler, Some(Func::args())));
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

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));
        self.functions.insert(name, (handler, Some(Func::args())));
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

        let handler: Arc<Handler> = Arc::new(move |stack, args| f(stack, args));
        self.functions.insert(name, (handler, None));
        Ok(())
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::VecDeque;
    ///
    /// #[derive(Debug, Clone)]
    /// struct StringQueue(VecDeque<String>);
    ///
    /// impl StringQueue {
    ///     fn new() -> Self {
    ///         Self(VecDeque::new())
    ///     }
    ///
    ///     fn len(&self) -> usize {
    ///         self.0.len()
    ///     }
    /// }
    ///
    /// runestick::decl_external!(StringQueue);
    ///
    /// # fn main() -> runestick::Result<()> {
    /// let mut module = runestick::Module::default();
    ///
    /// module.ty(&["StringQueue"]).build::<StringQueue>()?;
    /// module.function(&["StringQueue", "bytes"], StringQueue::new)?;
    /// module.inst_fn("len", StringQueue::len)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: InstFn<Args>,
    {
        let ty = Func::instance_value_type();
        let value_type_info = Func::instance_value_type_info();

        let key = (ty, name.to_hash());
        let name = name.to_name();

        if self.instance_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction {
                value_type_info,
                name,
            });
        }

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));

        let instance_function = ModuleInstanceFunction {
            handler,
            args: Some(Func::args()),
            value_type_info,
            name,
        };

        self.instance_functions.insert(key, instance_function);

        Ok(())
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::atomic::AtomicU32;
    /// use std::sync::Arc;
    ///
    /// runestick::decl_external!(MyType);
    ///
    /// #[derive(Clone, Debug)]
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
    /// module.ty(&["MyType"]).build::<MyType>()?;
    /// module.async_inst_fn("test", MyType::test)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_inst_fn<N, Func, Args>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        N: IntoInstFnHash,
        Func: AsyncInstFn<Args>,
    {
        let ty = Func::instance_value_type();
        let value_type_info = Func::instance_value_type_info();

        let key = (ty, name.to_hash());
        let name = name.to_name();

        if self.instance_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction {
                value_type_info,
                name,
            });
        }

        let handler: Arc<Handler> = Arc::new(move |stack, args| f.fn_call(stack, args));

        let instance_function = ModuleInstanceFunction {
            handler,
            args: Some(Func::args()),
            value_type_info,
            name,
        };

        self.instance_functions.insert(key, instance_function);
        Ok(())
    }
}

/// The builder for a type.
#[must_use = "must be consumed with build::<T>() to construct a type"]
pub struct TypeBuilder<'a, N> {
    name: N,
    types: &'a mut HashMap<ValueType, ModuleType>,
}

impl<N> TypeBuilder<'_, N>
where
    N: IntoIterator,
    N::Item: Into<Component>,
{
    /// Construct a new type, specifying which type it is with the parameter.
    pub fn build<T>(self) -> Result<(), ContextError>
    where
        T: ReflectValueType,
    {
        let name = Item::of(self.name);
        let value_type = T::value_type();
        let value_type_info = T::value_type_info();

        let ty = ModuleType {
            name: name.clone(),
            value_type_info,
        };

        if let Some(old) = self.types.insert(value_type, ty) {
            return Err(ContextError::ConflictingType {
                name,
                existing: old.value_type_info,
            });
        }

        Ok(())
    }
}

/// Trait used to provide the [function][Context::function] function.
pub trait Function<Args>: 'static + Copy + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_function][Context::async_function] function.
pub trait AsyncFunction<Args>: 'static + Copy + Send + Sync {
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [inst_fn][Context::inst_fn] function.
pub trait InstFn<Args>: 'static + Copy + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The owned type of the instance.
    type Owned;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type info of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

    /// Perform the vm call.
    fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_inst_fn][Context::async_inst_fn] function.
pub trait AsyncInstFn<Args>: 'static + Copy + Send + Sync {
    /// The type of the instance.
    type Instance;
    /// The owned type of the instance.
    type Owned;
    /// The return type of the function.
    type Return;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

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
                $(let $var = stack.pop()?;)*

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
                $(let $var = stack.pop()?;)*

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
            Instance: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Owned = <Instance as ReflectValueType>::Owned;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn instance_value_type() -> ValueType {
                Instance::value_type()
            }

            fn instance_value_type_info() -> ValueTypeInfo {
                Instance::value_type_info()
            }

            fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}
                let inst = stack.pop()?;
                $(let $var = stack.pop()?;)*

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
            Instance: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Owned = <Instance as ReflectValueType>::Owned;
            type Return = Return;

            fn args() -> usize {
                $count + 1
            }

            fn instance_value_type() -> ValueType {
                Instance::value_type()
            }

            fn instance_value_type_info() -> ValueTypeInfo {
                Instance::value_type_info()
            }

            fn fn_call(self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
                impl_register!{@check-args ($count + 1), args}
                let inst = stack.pop()?;
                $(let $var = stack.pop()?;)*

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
            Err(ValueError::Panic { reason }) => {
                return Err(VmError::from(VmErrorKind::Panic { reason }));
            },
            Err(ValueError::VmError { error }) => {
                return Err(*error);
            },
            Err(error) => {
                return Err(VmError::from(VmErrorKind::ReturnConversionError {
                    error,
                    ret: type_name::<$ty>()
                }));
            }
        };

        $stack.push($ret);
    };

    // Expand to function variable bindings.
    (@unsafe-vars $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = match <$ty>::unsafe_from_value($var) {
                Ok(v) => v,
                Err(ValueError::Panic { reason }) => {
                    return Err(VmError::from(VmErrorKind::Panic { reason }));
                },
                Err(ValueError::VmError { error }) => {
                    return Err(*error);
                },
                Err(error) => {
                    return Err(VmError::from(VmErrorKind::ArgumentConversionError {
                        error,
                        arg: $count - $num,
                        to: type_name::<$ty>(),
                    }));
                }
            };
        )*
    };

    // Expand to instance variable bindings.
    (@unsafe-inst-vars $inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = match Instance::unsafe_from_value($inst) {
            Ok(v) => v,
            Err(ValueError::Panic { reason }) => {
                return Err(VmError::from(VmErrorKind::Panic { reason }));
            },
            Err(ValueError::VmError { error }) => {
                return Err(*error);
            },
            Err(error) => {
                return Err(VmError::from(VmErrorKind::ArgumentConversionError {
                    error,
                    arg: 0,
                    to: type_name::<Instance>()
                }));
            }
        };

        $(
            let $var = match <$ty>::unsafe_from_value($var) {
                Ok(v) => v,
                Err(ValueError::Panic { reason }) => {
                    return Err(VmError::from(VmErrorKind::Panic { reason }));
                },
                Err(ValueError::VmError { error }) => {
                    return Err(*error);
                },
                Err(error) => {
                    return Err(VmError::from(VmErrorKind::ArgumentConversionError {
                        error,
                        arg: 1 + $count - $num,
                        to: type_name::<$ty>()
                    }));
                }
            };
        )*
    };

    (@check-args $expected:expr, $actual:expr) => {
        if $actual != $expected {
            return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                actual: $actual,
                expected: $expected,
            }));
        }
    };
}

impl_register!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
