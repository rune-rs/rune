use crate::collections::HashMap;
use crate::error;
use crate::hash::Hash;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{ExternalTypeError, ValueType, ValueTypeInfo};
use crate::vm::{StackError, Vm};
use std::any::type_name;
use std::fmt;
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
    /// Error raised in a user-defined function.
    #[error("error in user-defined function")]
    UserError {
        /// Cause of the error.
        #[from]
        error: error::Error,
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
    /// Wrong number of arguments provided in call.
    #[error("wrong number of arguments `{actual}`, expected `{expected}`")]
    ArgumentCountMismatch {
        /// The actual number of arguments.
        actual: usize,
        /// The expected number of arguments.
        expected: usize,
    },
}

/// Helper alias for boxed futures.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The handler of a function.
type Handler = dyn for<'vm> Fn(&'vm mut Vm, usize) -> BoxFuture<'vm, Result<(), CallError>> + Sync;

/// A description of a function signature.
#[derive(Debug)]
pub struct FnSignature {
    instance: Option<(&'static str, ValueType)>,
    name: String,
    args: usize,
}

impl FnSignature {
    /// Construct a new function signature.
    pub fn new_instance(instance: (&'static str, ValueType), name: &str, args: usize) -> Self {
        Self {
            instance: Some(instance),
            name: name.to_owned(),
            args,
        }
    }

    /// Construct a new global function signature.
    pub fn new_global(name: &str, args: usize) -> Self {
        Self {
            instance: None,
            name: name.to_owned(),
            args,
        }
    }
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((name, ty)) = self.instance {
            write!(fmt, "{} ({:?})::", name, ty)?;
        }

        write!(fmt, "{}(", self.name)?;

        let mut it = 0..self.args;
        let last = it.next_back();

        for _ in it {
            write!(fmt, "arg, ")?;
        }

        if last.is_some() {
            write!(fmt, "arg")?;
        }

        write!(fmt, ")")?;

        if self.instance.is_some() {
            write!(fmt, " (name: {})", Hash::of(&self.name))?;
        }

        Ok(())
    }
}

/// A collection of functions that can be looked up by type.
pub struct Functions {
    /// Free functions.
    functions: HashMap<Hash, Box<Handler>>,
    functions_info: HashMap<Hash, FnSignature>,
}

impl Functions {
    /// Construct a new functions container.
    pub fn new() -> Self {
        Self {
            functions: Default::default(),
            functions_info: Default::default(),
        }
    }

    /// Iterate over all available functions
    pub fn functions(&self) -> impl Iterator<Item = (Hash, &FnSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, RegisterError> {
        let mut functions = Self::new();
        crate::packages::core::install(&mut functions)?;
        crate::packages::bytes::install(&mut functions)?;
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
    /// functions.global_fallible_fn("empty", || Ok::<_, st::Error>(()))?;
    /// functions.global_fallible_fn("string", |a: String| Ok::<_, st::Error>(()))?;
    /// functions.global_fallible_fn("optional", |a: Option<String>| Ok::<_, st::Error>(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn global_fallible_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<Hash, RegisterError>
    where
        Func: GlobalFallibleFn<Args>,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        Ok(hash)
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
    ///
    ///     fn len(&self) -> usize {
    ///         self.0.len()
    ///     }
    /// }
    ///
    /// st::decl_external!(StringQueue);
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = st::Functions::new();
    ///
    /// functions.global_fn("bytes", StringQueue::new)?;
    /// functions.instance_fn("len", StringQueue::len)?;
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

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        self.functions_info
            .insert(hash, FnSignature::new_global(name, Func::args()));
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
    /// functions.async_fn("empty", || async { Ok::<_, st::Error>(()) })?;
    /// functions.async_fn("string", |a: String| async { Ok::<_, st::Error>(()) })?;
    /// functions.async_fn("optional", |a: Option<String>| async { Ok::<_, st::Error>(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<Hash, RegisterError>
    where
        Func: AsyncFn<Args>,
    {
        let hash = Hash::global_fn(name);

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| f.vm_call(vm, args));

        self.functions.insert(hash, handler);
        self.functions_info
            .insert(hash, FnSignature::new_global(name, Func::args()));
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
    /// functions.instance_fallible_fn("len", |s: &str| Ok::<_, st::Error>(s.len()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance_fallible_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<Hash, RegisterError>
    where
        Func: InstanceFallibleFn<Args>,
    {
        let ty = Func::instance_value_type();
        let hash = Hash::instance_fn(ty, Hash::of(name));

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        self.functions_info.insert(
            hash,
            FnSignature::new_instance((Func::instance_name(), ty), name, Func::args()),
        );
        Ok(hash)
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
    /// st::decl_external!(StringQueue);
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut functions = st::Functions::new();
    ///
    /// functions.global_fn("bytes", StringQueue::new)?;
    /// functions.instance_fn("len", StringQueue::len)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<Hash, RegisterError>
    where
        Func: InstanceFn<Args>,
    {
        let ty = Func::instance_value_type();
        let hash = Hash::instance_fn(ty, Hash::of(name));

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });

        self.functions.insert(hash, handler);
        self.functions_info.insert(
            hash,
            FnSignature::new_instance((Func::instance_name(), ty), name, Func::args()),
        );
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
    ///     async fn test(&self) -> st::Result<()> {
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
        let ty = Func::instance_value_type();
        let hash = Hash::instance_fn(ty, Hash::of(name));

        if self.functions.contains_key(&hash) {
            return Err(RegisterError::ConflictingFunction { hash });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| f.vm_call(vm, args));

        self.functions.insert(hash, handler);
        self.functions_info.insert(
            hash,
            FnSignature::new_instance((Func::instance_name(), ty), name, Func::args()),
        );
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
    pub fn raw_async_fn<F, O>(&mut self, name: &str, f: F) -> Result<Hash, RegisterError>
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

/// Trait used to provide the [global_fallible_fn][Functions::global_fallible_fn] function.
pub trait GlobalFallibleFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError>;
}

/// Trait used to provide the [global_fn][Functions::global_fn] function.
pub trait GlobalFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError>;
}

/// Trait used to provide the [async_fn][Self::async_fn] function.
pub trait AsyncFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), CallError>>;
}

/// Trait used to provide the [instance_fallible_fn][Functions::instance_fallible_fn] function.
pub trait InstanceFallibleFn<Args>: 'static + Copy + Send + Sync {
    /// The name of the instance for diagnostics purposes.
    fn instance_name() -> &'static str;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError>;
}

/// Trait used to provide the [instance_fn][Functions::instance_fn] function.
pub trait InstanceFn<Args>: 'static + Copy + Send + Sync {
    /// The name of the instance for diagnostics purposes.
    fn instance_name() -> &'static str;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError>;
}

/// Trait used to provide the [async_instance_fn][Functions::async_instance_fn] function.
pub trait AsyncInstanceFn<Args>: 'static + Copy + Send + Sync {
    /// The name of the instance for diagnostics purposes.
    fn instance_name() -> &'static str;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), CallError>>;
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
        impl<Func, Ret, Err, $($ty,)*> GlobalFallibleFn<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Result<Ret, Err>,
            Ret: ToValue,
            error::Error: From<Err>,
            $($ty: FromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError> {
                impl_register!{@args $count, args}

                $(let $var = vm.managed_pop()?;)*

                // Safety: We hold a reference to the Vm, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `vm`
                // when we return below.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@vars vm, $count, $($ty, $var, $num,)*}
                    self($($var,)*).map_err(error::Error::from)?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, $($ty,)*> GlobalFn<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Ret,
            Ret: ToValue,
            $($ty: FromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError> {
                impl_register!{@args $count, args}

                $(let $var = vm.managed_pop()?;)*

                // Safety: We hold a reference to the Vm, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `vm`
                // when we return below.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!{@vars vm, $count, $($ty, $var, $num,)*}
                    self($($var,)*)
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Output, Err, $($ty,)*> AsyncFn<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Ret,
            Ret: Future<Output = Result<Output, Err>>,
            Output: ToValue,
            error::Error: From<Err>,
            $($ty: UnsafeFromValue + ReflectValueType,)*
        {
            fn args() -> usize {
                $count
            }

            fn vm_call<'vm>(
                self,
                vm: &'vm mut Vm,
                args: usize
            ) -> BoxFuture<'vm, Result<(), CallError>> {
                Box::pin(async move {
                    impl_register!{@args $count, args}

                    $(let $var = vm.managed_pop()?;)*

                    // Safety: We hold a reference to the Vm, so we can
                    // guarantee that it won't be modified.
                    //
                    // The scope is also necessary, since we mutably access `vm`
                    // when we return below.
                    #[allow(unused_unsafe)]
                    let ret = unsafe {
                        impl_register!{@vars vm, $count, $($ty, $var, $num,)*}
                        self($($var,)*).await.map_err(error::Error::from)?
                    };

                    impl_register!{@return vm, ret, Ret}
                    Ok(())
                })
            }
        }

        impl<Func, Ret, Inst, Err, $($ty,)*> InstanceFallibleFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Result<Ret, Err>,
            Ret: ToValue,
            error::Error: From<Err>,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
            }

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError> {
                impl_register!{@args $count, args}

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
                    self(inst, $($var,)*).map_err(error::Error::from)?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Inst, $($ty,)*> InstanceFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Ret,
            Ret: ToValue,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
            }

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), CallError> {
                impl_register!{@args $count, args}

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
                    self(inst, $($var,)*)
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Output, Inst, $($ty,)*> AsyncInstanceFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Ret,
            Ret: Future<Output = Result<Output, error::Error>>,
            Output: ToValue,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
            }

            fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), CallError>> {
                Box::pin(async move {
                    impl_register!{@args $count, args}

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
                        self(inst, $($var,)*).await.map_err(CallError::from)?
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
                    to: type_name::<Inst>()
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

    (@args $expected:expr, $actual:expr) => {
        if $actual != $expected {
            return Err(CallError::ArgumentCountMismatch {
                actual: $actual,
                expected: $expected,
            });
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
