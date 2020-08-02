use crate::collections::HashMap;
use crate::hash::Hash;
use crate::reflection::{ReflectValueType, ToValue, UnsafeFromValue};
use crate::tls;
use crate::value::{ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};
use std::any::type_name;
use std::future::Future;

use crate::context::item::Item;
use crate::context::{BoxFuture, ContextError, FnSignature, Handler};

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct Module {
    /// The name of the module.
    pub(super) path: Item,
    /// Free functions.
    pub(super) functions: HashMap<Item, (Handler, FnSignature)>,
    /// Instance functions.
    pub(super) instance_functions: HashMap<(ValueType, String), (Handler, FnSignature)>,
    /// Registered types.
    pub(super) types: HashMap<ValueType, (ValueTypeInfo, Item)>,
}

impl Module {
    /// Construct a new module.
    pub fn new<I>(path: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        Self {
            path: Item::of(path),
            functions: Default::default(),
            instance_functions: Default::default(),
            types: Default::default(),
        }
    }

    /// Register a type.
    ///
    /// This will allow the type to be used within scripts, using the item named
    /// here.
    pub fn ty<N>(&mut self, name: N) -> TypeBuilder<'_, N>
    where
        N: IntoIterator,
        N::Item: AsRef<str>,
    {
        TypeBuilder {
            name,
            path: &self.path,
            types: &mut self.types,
        }
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
    /// st::decl_external!(StringQueue);
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.function(&["bytes"], StringQueue::new)?;
    /// module.function(&["empty"], || Ok::<_, st::Error>(()))?;
    /// module.function(&["string"], |a: String| Ok::<_, st::Error>(()))?;
    /// module.function(&["optional"], |a: Option<String>| Ok::<_, st::Error>(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: Function<Args>,
        N: IntoIterator,
        N::Item: AsRef<str>,
    {
        let name = self.path.join(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        let handler = Handler::Regular(Box::new(move |vm, args| f.vm_call(vm, args)));
        let signature = FnSignature::new_free(self.path.join(&name), Func::args());
        self.functions.insert(name, (handler, signature));
        Ok(())
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.async_function(&["empty"], || async { () })?;
    /// module.async_function(&["empty_fallible"], || async { Ok::<_, st::Error>(()) })?;
    /// module.async_function(&["string"], |a: String| async { Ok::<_, st::Error>(()) })?;
    /// module.async_function(&["optional"], |a: Option<String>| async { Ok::<_, st::Error>(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_function<Func, Args, N>(&mut self, name: N, f: Func) -> Result<(), ContextError>
    where
        Func: AsyncFunction<Args>,
        N: IntoIterator,
        N::Item: AsRef<str>,
    {
        let name = self.path.join(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        let handler = Handler::Async(Box::new(move |vm, args| f.vm_call(vm, args)));
        let signature = FnSignature::new_free(self.path.join(&name), Func::args());
        self.functions.insert(name, (handler, signature));
        Ok(())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_fn<F, N>(&mut self, name: N, f: F) -> Result<(), ContextError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), VmError> + Send + Sync,
        N: IntoIterator,
        N::Item: AsRef<str>,
    {
        let name = self.path.join(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let handler = Handler::Regular(Box::new(move |vm, args| f(vm, args)));
        let signature = FnSignature::new_raw(self.path.join(&name));
        self.functions.insert(name.to_owned(), (handler, signature));
        Ok(())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn async_raw_fn<F, O, N>(&mut self, name: N, f: F) -> Result<(), ContextError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> O + Send + Sync,
        O: Future<Output = Result<(), VmError>>,
        N: IntoIterator,
        N::Item: AsRef<str>,
    {
        let name = Item::of(name);

        if self.functions.contains_key(&name) {
            return Err(ContextError::ConflictingFunctionName { name });
        }

        let handler = Handler::Async(Box::new(move |vm, args| {
            Box::pin(async move { f(vm, args).await })
        }));
        let signature = FnSignature::new_raw(self.path.join(&name));
        self.functions.insert(name, (handler, signature));
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
    /// st::decl_external!(StringQueue);
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.ty(&["StringQueue"]).build::<StringQueue>()?;
    /// module.function(&["StringQueue", "bytes"], StringQueue::new)?;
    /// module.inst_fn("len", StringQueue::len)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn inst_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<(), ContextError>
    where
        Func: InstFn<Args>,
    {
        let ty = Func::instance_value_type();
        let type_info = Func::instance_value_type_info();

        let key = (ty, name.to_owned());

        if self.instance_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction {
                type_info,
                name: name.to_owned(),
            });
        }

        let handler = Handler::Regular(Box::new(move |vm, args| f.vm_call(vm, args)));
        let signature = FnSignature::new_inst(type_info, name, Func::args());

        self.instance_functions
            .insert(key.clone(), (handler, signature));
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
    /// let mut module = st::Module::default();
    ///
    /// module.ty(&["MyType"]).build::<MyType>()?;
    /// module.async_inst_fn("test", MyType::test)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_inst_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<(), ContextError>
    where
        Func: AsyncInstFn<Args>,
    {
        let ty = Func::instance_value_type();
        let type_info = Func::instance_value_type_info();

        let key = (ty, name.to_owned());

        if self.instance_functions.contains_key(&key) {
            return Err(ContextError::ConflictingInstanceFunction {
                type_info,
                name: name.to_owned(),
            });
        }

        let handler = Handler::Async(Box::new(move |vm, args| f.vm_call(vm, args)));
        let signature = FnSignature::new_inst(type_info, name, Func::args());

        self.instance_functions
            .insert(key.clone(), (handler, signature));
        Ok(())
    }
}

/// The builder for a type.
#[must_use = "must be consumed with build::<T>() to construct a type"]
pub struct TypeBuilder<'a, N> {
    name: N,
    path: &'a Item,
    types: &'a mut HashMap<ValueType, (ValueTypeInfo, Item)>,
}

impl<N> TypeBuilder<'_, N>
where
    N: IntoIterator,
    N::Item: AsRef<str>,
{
    /// Construct a new type, specifying which type it is with the parameter.
    pub fn build<T>(self) -> Result<(), ContextError>
    where
        T: ReflectValueType,
    {
        let name = self.path.join(self.name);
        let value_type = T::value_type();
        let type_info = T::value_type_info();

        if let Some((existing, _)) = self.types.insert(value_type, (type_info, name.clone())) {
            let hash = Hash::of_type(&name);

            return Err(ContextError::ConflictingType {
                name,
                hash,
                existing,
            });
        }

        Ok(())
    }
}

/// Helper trait to convert function return values into results.
pub trait IntoVmResult {
    type Output: ToValue;

    fn into_vm_result(self) -> Result<Self::Output, VmError>;
}

impl<T> IntoVmResult for T
where
    T: ToValue,
{
    type Output = T;

    fn into_vm_result(self) -> Result<Self::Output, VmError> {
        Ok(self)
    }
}

impl<T, E> IntoVmResult for Result<T, E>
where
    crate::Error: From<E>,
    T: ToValue,
{
    type Output = T;

    fn into_vm_result(self) -> Result<Self::Output, VmError> {
        use crate::error::Error;
        self.map_err(|e| VmError::from(Error::from(e)))
    }
}

/// Trait used to provide the [function][Context::function] function.
pub trait Function<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_function][Context::async_function] function.
pub trait AsyncFunction<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), VmError>>;
}

/// Trait used to provide the [inst_fn][Context::inst_fn] function.
pub trait InstFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type info of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_inst_fn][Context::async_inst_fn] function.
pub trait AsyncInstFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), VmError>>;
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
        impl<Func, Ret, $($ty,)*> Function<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Ret,
            Ret: IntoVmResult,
            Ret::Output: ToValue,
            $($ty: UnsafeFromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn vm_call(
                self,
                vm: &mut Vm,
                args: usize
            ) -> Result<(), VmError> {
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
                    tls::inject_vm(vm, || self($($var.0,)*)).into_vm_result()?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, $($ty,)*> AsyncFunction<($($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn($($ty,)*) -> Ret,
            Ret: Future,
            Ret::Output: IntoVmResult,
            $($ty: UnsafeFromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn vm_call<'vm>(
                self,
                vm: &'vm mut Vm,
                args: usize
            ) -> BoxFuture<'vm, Result<(), VmError>> {
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
                        tls::InjectVm::new(vm, self($($var.0,)*)).await.into_vm_result()?
                    };

                    impl_register!{@return vm, ret, Ret}
                    Ok(())
                })
            }
        }

        impl<Func, Ret, Inst, $($ty,)*> InstFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Ret,
            Ret: IntoVmResult,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::value_type()
            }

            fn instance_value_type_info() -> ValueTypeInfo {
                Inst::value_type_info()
            }

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError> {
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
                    tls::inject_vm(vm, || self(inst.0, $($var.0,)*)).into_vm_result()?
                };

                impl_register!{@return vm, ret, Ret}
                Ok(())
            }
        }

        impl<Func, Ret, Inst, $($ty,)*> AsyncInstFn<(Inst, $($ty,)*)> for Func
        where
            Func: 'static + Copy + Send + Sync + Fn(Inst $(, $ty)*) -> Ret,
            Ret: Future,
            Ret::Output: IntoVmResult,
            Inst: UnsafeFromValue + ReflectValueType,
            $($ty: UnsafeFromValue,)*
        {
            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::value_type()
            }

            fn instance_value_type_info() -> ValueTypeInfo {
                Inst::value_type_info()
            }

            fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), VmError>> {
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
                        tls::InjectVm::new(vm, self(inst.0, $($var.0,)*)).await.into_vm_result()?
                    };

                    impl_register!{@return vm, ret, Ret}
                    Ok(())
                })
            }
        }
    };

    (@return $vm:ident, $ret:ident, $ty:ty) => {
        let $ret = match $ret.to_value($vm) {
            Ok($ret) => $ret,
            Err(error) => {
                return Err(VmError::ReturnConversionError {
                    error,
                    ret: type_name::<$ty>()
                });
            }
        };

        $vm.unmanaged_push($ret);
    };

    // Expand to function variable bindings.
    (@vars $vm:expr, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = match <$ty>::unsafe_from_value($var, $vm) {
                Ok(v) => v,
                Err(error) => {
                    let ty = $var.type_info($vm)?;

                    return Err(VmError::ArgumentConversionError {
                        error,
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
            Err(error) => {
                let ty = $inst.type_info($vm)?;

                return Err(VmError::ArgumentConversionError {
                    error,
                    arg: 0,
                    from: ty,
                    to: type_name::<Inst>()
                });
            }
        };

        $(
            let $var = match <$ty>::unsafe_from_value($var, $vm) {
                Ok(v) => v,
                Err(error) => {
                    let ty = $var.type_info($vm)?;

                    return Err(VmError::ArgumentConversionError {
                        error,
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
            return Err(VmError::ArgumentCountMismatch {
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
