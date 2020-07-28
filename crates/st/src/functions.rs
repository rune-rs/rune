use crate::collections::HashMap;
use crate::hash::Hash;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{ExternalTypeError, ValueType, ValueTypeInfo};
use crate::vm::{StackError, Vm};
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

/// An error raised while registering a function.
#[derive(Debug, Error)]
pub enum RegisterError {
    /// Error raised when attempting to register a conflicting function.
    #[error("function with hash `{hash}` already exists")]
    ConflictingFunction {
        /// The hash of the conflicting function.
        hash: Hash,
    },
}

/// An error raised during a function call.
#[derive(Debug, Error)]
pub enum CallError {
    /// Other boxed error raised.
    #[error("other error")]
    Other {
        /// The error raised.
        error: anyhow::Error,
    },
    /// Failure to interact with the stack.
    #[error("failed to interact with the stack")]
    StackError {
        /// Source error.
        #[from]
        error: StackError,
    },
    /// Failure to resolve external type.
    #[error("failed to resolve type info for external type")]
    ExternalTypeError {
        /// Source error.
        #[from]
        error: ExternalTypeError,
    },
    /// Failure to convert from one type to another.
    #[error("failed to convert argument #{arg} from `{from}` to `{to}`")]
    ArgumentConversionError {
        /// The argument location that was converted.
        arg: usize,
        /// The value type we attempted to convert from.
        from: ValueTypeInfo,
        /// The native type we attempt to convert to.
        to: &'static str,
    },
    /// Failure to convert return value.
    #[error("failed to convert return value `{ret}`")]
    ReturnConversionError {
        /// Type of the return value we attempted to convert.
        ret: &'static str,
    },
}

impl CallError {
    /// Construct a boxed error.
    pub fn other<E>(error: E) -> Self
    where
        E: 'static + std::error::Error + Send + Sync,
    {
        Self::Other {
            error: error.into(),
        }
    }
}

/// Helper alias for boxed futures.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The handler of a function.
type Handler = dyn for<'vm> Fn(&'vm mut Vm, usize) -> BoxFuture<'vm, Result<(), CallError>> + Sync;

/// A collection of functions that can be looked up by type.
pub struct Functions {
    /// Free functions.
    functions: HashMap<Hash, Box<Handler>>,
}

impl Functions {
    /// Construct a new functions container.
    pub fn new() -> Self {
        Self {
            functions: Default::default(),
        }
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, RegisterError> {
        let mut functions = Self::new();
        crate::packages::core::install(&mut functions)?;
        Ok(functions)
    }

    /// Lookup the given function.
    pub fn lookup(&self, hash: Hash) -> Option<&Handler> {
        let handler = self.functions.get(&hash)?;
        Some(&*handler)
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = st::Functions::new();
    ///
    /// functions.global_fn("empty", || Ok(()))?;
    /// functions.global_fn("string", |a: String| Ok(()))?;
    /// functions.global_fn("optional", |a: Option<String>| Ok(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn global_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<Hash, RegisterError>
    where
        Func: GlobalFn<Args>,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, _| {
            let ret = f.vm_call(vm);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        Ok(hash)
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use st::Functions;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = Functions::new();
    ///
    /// functions.async_global_fn("empty", || async { Ok(()) })?;
    /// functions.async_global_fn("string", |a: String| async { Ok(()) })?;
    /// functions.async_global_fn("optional", |a: Option<String>| async { Ok(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_global_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<Hash, RegisterError>
    where
        Func: AsyncGlobalFn<Args>,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, _| f.vm_call(vm));

        self.functions.insert(hash, handler);
        Ok(hash)
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = st::Functions::new();
    ///
    /// functions.instance_fn("len", |s: &str| Ok(s.len()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<Hash, RegisterError>
    where
        Func: InstanceFn<Args>,
    {
        let hash = Hash::instance_fn(Func::instance_value_type(), name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, _| {
            let ret = f.vm_call(vm);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        Ok(hash)
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::atomic::AtomicU32;
    /// use std::sync::Arc;
    ///
    /// st::decl_external!(MyType);
    ///
    /// #[derive(Clone, Debug)]
    /// struct MyType {
    ///     value: Arc<AtomicU32>,
    /// }
    ///
    /// impl MyType {
    ///     async fn test(&self) -> Result<(), st::CallError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = st::Functions::new();
    /// functions.async_instance_fn("test", MyType::test)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_instance_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<Hash, RegisterError>
    where
        Func: AsyncInstanceFn<Args>,
    {
        let hash = Hash::instance_fn(Func::instance_value_type(), name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, _| f.vm_call(vm));

        self.functions.insert(hash, handler);
        Ok(hash)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_global_fn<F>(&mut self, name: &str, f: F) -> Result<Hash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), CallError> + Send + Sync,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        self.functions.insert(
            hash,
            Box::new(move |vm, args| Box::pin(async move { f(vm, args) })),
        );

        Ok(hash)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_async_global_fn<F, O>(&mut self, name: &str, f: F) -> Result<Hash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> O + Send + Sync,
        O: Future<Output = Result<(), CallError>>,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        self.functions.insert(
            hash,
            Box::new(move |vm, args| Box::pin(async move { f(vm, args).await })),
        );

        Ok(hash)
    }
}

/// Trait used to provide the [global_fn][Functions::global_fn] function.
pub trait GlobalFn<Args>: 'static + Copy + Send + Sync {
    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm) -> Result<(), CallError>;
}

/// Trait used to provide the [async_global_fn][Self::async_global_fn] function.
pub trait AsyncGlobalFn<Args>: 'static + Copy + Send + Sync {
    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm) -> BoxFuture<'vm, Result<(), CallError>>;
}

/// Trait used to provide the [instance_fn][Functions::instance_fn] function.
pub trait InstanceFn<Args>: 'static + Copy + Send + Sync {
    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm) -> Result<(), CallError>;
}

/// Trait used to provide the [async_instance_fn][Functions::async_instance_fn] function.
pub trait AsyncInstanceFn<Args>: 'static + Copy + Send + Sync {
    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm) -> BoxFuture<'vm, Result<(), CallError>>;
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
        impl<Func, Ret, $($ty,)*> GlobalFn<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Result<Ret, CallError>,
            Ret: ToValue,
            $($ty: FromValue,)*
        {
            fn vm_call(self, vm: &mut Vm) -> Result<(), CallError> {
                $(let $var = vm.managed_pop()?;)*

                // Safety: We hold a reference to the Vm, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `vm`
                // when we return below.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@vars vm, $count, $($ty, $var, $num,)*}
                    self($($var,)*)?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Output, $($ty,)*> AsyncGlobalFn<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Ret,
            Ret: Future<Output = Result<Output, CallError>>,
            Output: ToValue,
            $($ty: UnsafeFromValue + ReflectValueType,)*
        {
            fn vm_call<'vm>(
                self,
                vm: &'vm mut Vm,
            ) -> BoxFuture<'vm, Result<(), CallError>> {
                Box::pin(async move {
                    $(let $var = vm.managed_pop()?;)*

                    // Safety: We hold a reference to the Vm, so we can
                    // guarantee that it won't be modified.
                    //
                    // The scope is also necessary, since we mutably access `vm`
                    // when we return below.
                    #[allow(unused_unsafe)]
                    let ret = unsafe {
                        impl_register!{@vars vm, $count, $($ty, $var, $num,)*}
                        self($($var,)*).await?
                    };

                    impl_register!{@return vm, ret, Ret}
                    Ok(())
                })
            }
        }

        impl<Func, Ret, Inst, $($ty,)*> InstanceFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Result<Ret, CallError>,
            Ret: ToValue,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
            }

            fn vm_call(self, vm: &mut Vm) -> Result<(), CallError> {
                let inst = vm.managed_pop()?;
                $(let $var = vm.managed_pop()?;)*

                // Safety: We hold a reference to the Vm, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `vm`
                // when we return below.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@unsafeinstancevars inst, vm, $count, $($ty, $var, $num,)*}
                    self(inst, $($var,)*)?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Output, Inst, $($ty,)*> AsyncInstanceFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Ret,
            Ret: Future<Output = Result<Output, CallError>>,
            Output: ToValue,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
            }

            fn vm_call<'vm>(self, vm: &'vm mut Vm) -> BoxFuture<'vm, Result<(), CallError>> {
                Box::pin(async move {
                    let inst = vm.managed_pop()?;
                    $(let $var = vm.managed_pop()?;)*

                    // Safety: We hold a reference to the Vm, so we can
                    // guarantee that it won't be modified.
                    //
                    // The scope is also necessary, since we mutably access `vm`
                    // when we return below.
                    #[allow(unused_unsafe)]
                    let ret = unsafe {
                        impl_register!{@unsafeinstancevars inst, vm, $count, $($ty, $var, $num,)*}
                        self(inst, $($var,)*).await?
                    };

                    impl_register!{@return vm, ret, Ret}
                    Ok(())
                })
            }
        }
    };

    (@return $vm:ident, $ret:ident, $ty:ty) => {
        let $ret = match $ret.to_value($vm) {
            Some($ret) => $ret,
            None => {
                return Err(CallError::ReturnConversionError {
                    ret: type_name::<$ty>()
                });
            }
        };

        $vm.managed_push($ret)?;
    };

    // Expand to function variable bindings.
    (@vars $vm:expr, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = match <$ty>::unsafe_from_value($var, $vm) {
                Ok(v) => v,
                Err(v) => {
                    let ty = v.type_info($vm)?;

                    return Err(CallError::ArgumentConversionError {
                        arg: $count - $num,
                        from: ty,
                        to: type_name::<$ty>(),
                    });
                }
            };
        )*
    };

    // Expand to instance variable bindings.
    (@unsafeinstancevars $inst:ident, $vm:expr, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = match Inst::unsafe_from_value($inst, $vm) {
            Ok(v) => v,
            Err(v) => {
                let ty = v.type_info($vm)?;

                return Err(CallError::ArgumentConversionError {
                    arg: 0,
                    from: ty,
                    to: type_name::<&Inst>()
                });
            }
        };

        $(
            let $var = match <$ty>::unsafe_from_value($var, $vm) {
                Ok(v) => v,
                Err(v) => {
                    let ty = v.type_info($vm)?;

                    return Err(CallError::ArgumentConversionError {
                        arg: 1 + $count - $num,
                        from: ty,
                        to: type_name::<$ty>()
                    });
                }
            };
        )*
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
