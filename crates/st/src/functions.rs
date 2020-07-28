use crate::collections::HashMap;
use crate::hash::{FnHash, Hash};
use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ExternalTypeError, ValueTypeInfo};
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
        hash: FnHash,
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

/// The handler of a function.
type FnHandler = dyn (for<'vm> Fn(&'vm mut Vm, usize) -> Pin<Box<dyn Future<Output = Result<(), CallError>> + 'vm>>)
    + Send
    + Sync;

/// A collection of functions that can be looked up by type.
pub struct Functions {
    handlers: HashMap<FnHash, Box<FnHandler>>,
}

impl Functions {
    /// Construct a new functions container.
    pub fn new() -> Self {
        Self {
            handlers: Default::default(),
        }
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, RegisterError> {
        let mut functions = Self::new();
        crate::packages::core::install(&mut functions)?;
        Ok(functions)
    }

    /// Lookup the given function.
    pub fn lookup(&self, hash: FnHash) -> Option<&FnHandler> {
        let handler = self.handlers.get(&hash)?;
        Some(&*handler)
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use st::{Functions, Register as _};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = Functions::new();
    ///
    /// functions.register("empty", || Ok(()))?;
    /// functions.register("string", |a: String| Ok(()))?;
    /// functions.register("optional", |a: Option<String>| Ok(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register<Func, Args>(&mut self, name: &str, f: Func) -> Result<FnHash, RegisterError>
    where
        Func: Register<Args>,
    {
        let hash = Func::hash(name);

        if self.handlers.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<FnHandler> = Box::new(move |vm, _| {
            Box::pin(async move {
                f.vm_call(vm)?;
                Ok(())
            })
        });

        self.handlers.insert(hash, handler);
        Ok(hash)
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use st::{Functions, RegisterAsync as _};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = Functions::new();
    ///
    /// functions.register_async("empty", || async { Ok(()) })?;
    /// functions.register_async("string", |a: String| async { Ok(()) })?;
    /// functions.register_async("optional", |a: Option<String>| async { Ok(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_async<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<FnHash, RegisterError>
    where
        Func: RegisterAsync<Args>,
    {
        let hash = Func::hash(name);

        if self.handlers.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<FnHandler> = Box::new(move |vm, _| f.vm_call(vm));

        self.handlers.insert(hash, handler);
        Ok(hash)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn register_raw<F>(&mut self, name: &str, f: F) -> Result<FnHash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), CallError> + Send + Sync,
    {
        let hash = FnHash::raw(Hash::of(name));

        if self.handlers.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        self.handlers.insert(
            hash,
            Box::new(move |vm, args| Box::pin(async move { f(vm, args) })),
        );

        Ok(hash)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn register_raw_async<F, O>(&mut self, name: &str, f: F) -> Result<FnHash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> O + Send + Sync,
        O: Future<Output = Result<(), CallError>>,
    {
        let hash = FnHash::raw(Hash::of(name));

        if self.handlers.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        self.handlers.insert(
            hash,
            Box::new(move |vm, args| Box::pin(async move { f(vm, args).await })),
        );

        Ok(hash)
    }
}

/// Trait used to provide the [register][Self::register] function.
pub trait Register<Args>: 'static + Copy + Send + Sync {
    /// Get the signature hash of the function.
    fn hash(name: &str) -> FnHash;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm) -> Result<(), CallError>;
}

/// Trait used to provide the [register][Self::register] function.
pub trait RegisterAsync<Args>: 'static + Copy + Send + Sync {
    /// Get the signature hash of the function.
    fn hash(name: &str) -> FnHash;

    /// Perform the vm call.
    fn vm_call<'vm>(
        self,
        vm: &'vm mut Vm,
    ) -> Pin<Box<dyn Future<Output = Result<(), CallError>> + 'vm>>;
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
        impl<Func, Ret, $($ty,)*> Register<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + (Fn($($ty,)*) -> Result<Ret, CallError>),
            Ret: ToValue,
            $($ty: FromValue + ReflectValueType,)*
        {
            fn hash(name: &str) -> FnHash {
                FnHash::of(name, &[$($ty::reflect_value_type(),)*])
            }

            fn vm_call(self, vm: &mut Vm) -> Result<(), CallError> {
                $(
                    let $var = vm.managed_pop()?;

                    let $var = match $ty::from_value($var, vm) {
                        Ok(v) => v,
                        Err(v) => {
                            let ty = v.type_info(vm)?;

                            return Err(CallError::ArgumentConversionError {
                                arg: $count - $num,
                                from: ty,
                                to: type_name::<$ty>()
                            });
                        }
                    };
                )*

                let ret = self($($var,)*)?;
                let ret = ret.to_value(vm).unwrap();
                vm.managed_push(ret)?;
                Ok(())
            }
        }

        impl<Func, Ret, Output, $($ty,)*> RegisterAsync<($($ty,)*)> for Func
        where
            Func: 'static + Copy + (Fn($($ty,)*) -> Ret) + Send + Sync,
            Ret: Future<Output = Result<Output, CallError>>,
            Output: ToValue,
            $($ty: FromValue + ReflectValueType,)*
        {
            fn hash(name: &str) -> FnHash {
                FnHash::of(name, &[$($ty::reflect_value_type(),)*])
            }

            fn vm_call<'vm>(
                self,
                vm: &'vm mut Vm,
            ) -> Pin<Box<dyn Future<Output = Result<(), CallError>> + 'vm>> {
                Box::pin(async move {
                    $(
                        let $var = vm.managed_pop()?;
                        let $var = match $ty::from_value($var, vm) {
                            Ok(v) => v,
                            Err(v) => {
                                let ty = v.type_info(vm)?;

                                return Err(CallError::ArgumentConversionError {
                                    arg: $count - $num,
                                    from: ty,
                                    to: type_name::<$ty>(),
                                });
                            }
                        };
                    )*

                    let ret = self($($var,)*).await?;
                    let ret = ret.to_value(vm).unwrap();
                    vm.managed_push(ret)?;
                    Ok(())
                })
            }
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
