use crate::collections::HashMap;
use crate::error;
use crate::hash::Hash;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};
use std::any::type_name;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

/// An error raised while registering a function.
#[derive(Debug, Error)]
pub enum RegisterError {
    /// Error raised when attempting to register a conflicting function.
    #[error("function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        /// The signature of the conflicting function.
        signature: FnSignature,
        /// The hash of the conflicting function.
        hash: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function with name `{name}` already exists")]
    ConflictingFunctionName {
        /// The name of the conflicting function.
        name: String,
    },
    /// Error raised when attempting to register a conflicting instance function.
    #[error("instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction {
        /// Type that we register the instance function for.
        type_info: ValueTypeInfo,
        /// The name of the conflicting function.
        name: String,
    },
    /// Tried to insert a module that conflicted with an already existing one.
    #[error("module `{name}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        name: ItemPath,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
}

/// The name of a module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ItemPath {
    path: Vec<String>,
}

impl ItemPath {
    /// Construct a new item path.
    pub fn new(path: Vec<String>) -> Self {
        Self { path }
    }

    /// Access the last component in the path.
    pub fn last(&self) -> Option<&str> {
        self.path.last().map(String::as_str)
    }

    /// Construct a new item path.
    pub fn of<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        Self {
            path: iter
                .into_iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Vec<_>>(),
        }
    }

    /// Return the hash of the specified function.
    pub fn hash_function(&self, function: &str) -> Hash {
        Hash::function(self.path.iter().map(String::as_str).chain(Some(function)))
    }
}

impl fmt::Display for ItemPath {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.path.iter().peekable();

        while let Some(part) = it.next() {
            write!(fmt, "{}", part)?;

            if it.peek().is_some() {
                write!(fmt, "::")?;
            }
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a ItemPath {
    type IntoIter = std::slice::Iter<'a, String>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.path.iter()
    }
}

/// Helper alias for boxed futures.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The handler of a function.
type Handler = dyn for<'vm> Fn(&'vm mut Vm, usize) -> BoxFuture<'vm, Result<(), VmError>> + Sync;

/// A description of a function signature.
#[derive(Debug, Clone)]
pub struct FnSignature {
    path: Arc<ItemPath>,
    instance: Option<ValueTypeInfo>,
    name: String,
    args: Option<usize>,
}

impl FnSignature {
    /// Construct a new function signature.
    pub fn new_instance(
        path: Arc<ItemPath>,
        instance: ValueTypeInfo,
        name: &str,
        args: usize,
    ) -> Self {
        Self {
            path,
            instance: Some(instance),
            name: name.to_owned(),
            args: Some(args),
        }
    }

    /// Construct a new raw signature.
    pub fn new_raw(path: Arc<ItemPath>, name: &str) -> Self {
        Self {
            path,
            instance: None,
            name: name.to_owned(),
            args: None,
        }
    }

    /// Construct a new global function signature.
    pub fn new_global(path: Arc<ItemPath>, name: &str, args: usize) -> Self {
        Self {
            path,
            instance: None,
            name: name.to_owned(),
            args: Some(args),
        }
    }
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(instance) = self.instance {
            write!(fmt, "<{}>::{}(self", instance, self.name)?;

            if let Some(args) = self.args {
                for n in 0..args {
                    write!(fmt, ", #{}", n)?;
                }
            } else {
                write!(fmt, ", ...")?;
            }

            write!(fmt, ")")?;
        } else {
            write!(fmt, "{}::{}(", self.path, self.name)?;

            if let Some(args) = self.args {
                let mut it = 0..args;
                let last = it.next_back();

                for n in it {
                    write!(fmt, "#{}, ", n)?;
                }

                if let Some(n) = last {
                    write!(fmt, "#{}", n)?;
                }
            } else {
                write!(fmt, "...")?;
            }

            write!(fmt, ")")?;
        }
        Ok(())
    }
}

/// Functions visible to the virtual machine.
#[derive(Default)]
pub struct Functions {
    /// Free functions.
    functions: HashMap<Hash, Box<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
}

impl Functions {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Functions::default()
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, RegisterError> {
        let mut this = Self::new();
        this.install(crate::packages::core::module()?)?;
        this.install(crate::packages::bytes::module()?)?;
        this.install(crate::packages::string::module()?)?;
        Ok(this)
    }

    /// Iterate over all available functions
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &FnSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Lookup the given function.
    pub fn lookup(&self, hash: Hash) -> Option<&Handler> {
        let handler = self.functions.get(&hash)?;
        Some(&*handler)
    }

    /// Install the specified module.
    pub fn install(&mut self, module: Module) -> Result<(), RegisterError> {
        let base = module.path;

        for (name, (handler, signature)) in module.functions.into_iter() {
            let hash = Hash::function(base.into_iter().chain(Some(&name)));

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(RegisterError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        for ((ty, name), (handler, signature)) in module.instance_functions.into_iter() {
            let hash = Hash::instance_function(ty, Hash::of(name));

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(RegisterError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        Ok(())
    }
}

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct Module {
    /// The name of the module.
    path: Arc<ItemPath>,
    /// Free functions.
    functions: HashMap<String, (Box<Handler>, FnSignature)>,
    /// Instance functions.
    instance_functions: HashMap<(ValueType, String), (Box<Handler>, FnSignature)>,
}

impl Module {
    /// Construct a new module.
    pub fn new<I>(path: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        Self {
            path: Arc::new(ItemPath::of(path)),
            functions: Default::default(),
            instance_functions: Default::default(),
        }
    }

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.global_fallible_fn("empty", || Ok::<_, st::Error>(()))?;
    /// module.global_fallible_fn("string", |a: String| Ok::<_, st::Error>(()))?;
    /// module.global_fallible_fn("optional", |a: Option<String>| Ok::<_, st::Error>(()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn global_fallible_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<(), RegisterError>
    where
        Func: GlobalFallibleFn<Args>,
    {
        if self.functions.contains_key(name) {
            return Err(RegisterError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });

        let signature = FnSignature::new_global(self.path.clone(), name, Func::args());
        self.functions.insert(name.to_owned(), (handler, signature));
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
    /// st::decl_external!(StringQueue);
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.global_fn("bytes", StringQueue::new)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn global_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<(), RegisterError>
    where
        Func: GlobalFn<Args>,
    {
        if self.functions.contains_key(name) {
            return Err(RegisterError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });
        let signature = FnSignature::new_global(self.path.clone(), name, Func::args());

        self.functions.insert(name.to_owned(), (handler, signature));
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
    /// module.async_fn("empty", || async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("string", |a: String| async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("optional", |a: Option<String>| async { Ok::<_, st::Error>(()) })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<(), RegisterError>
    where
        Func: AsyncFn<Args>,
    {
        if self.functions.contains_key(name) {
            return Err(RegisterError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| f.vm_call(vm, args));
        let signature = FnSignature::new_global(self.path.clone(), name, Func::args());

        self.functions.insert(name.to_owned(), (handler, signature));
        Ok(())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_fn<F>(&mut self, name: &str, f: F) -> Result<(), RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), VmError> + Send + Sync,
    {
        if self.functions.contains_key(name) {
            return Err(RegisterError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let signature = FnSignature::new_raw(self.path.clone(), name);
        let handler: Box<Handler> = Box::new(move |vm, args| Box::pin(async move { f(vm, args) }));
        self.functions.insert(name.to_owned(), (handler, signature));
        Ok(())
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_async_fn<F, O>(&mut self, name: &str, f: F) -> Result<(), RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> O + Send + Sync,
        O: Future<Output = Result<(), VmError>>,
    {
        if self.functions.contains_key(name) {
            return Err(RegisterError::ConflictingFunctionName {
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> =
            Box::new(move |vm, args| Box::pin(async move { f(vm, args).await }));
        let signature = FnSignature::new_raw(self.path.clone(), name);

        self.functions.insert(name.to_owned(), (handler, signature));

        Ok(())
    }

    /// Register an instance function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.instance_fallible_fn("len", |s: &str| Ok::<_, st::Error>(s.len()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance_fallible_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<(), RegisterError>
    where
        Func: InstanceFallibleFn<Args>,
    {
        let ty = Func::instance_value_type();
        let type_info = Func::instance_value_type_info();

        let key = (ty, name.to_owned());

        if self.instance_functions.contains_key(&key) {
            return Err(RegisterError::ConflictingInstanceFunction {
                type_info,
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });
        let signature = FnSignature::new_instance(self.path.clone(), type_info, name, Func::args());

        self.instance_functions
            .insert(key.clone(), (handler, signature));
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
    /// module.global_fn("bytes", StringQueue::new)?;
    /// module.instance_fn("len", StringQueue::len)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance_fn<Func, Args>(&mut self, name: &str, f: Func) -> Result<(), RegisterError>
    where
        Func: InstanceFn<Args>,
    {
        let ty = Func::instance_value_type();
        let type_info = Func::instance_value_type_info();

        let key = (ty, name.to_owned());

        if self.instance_functions.contains_key(&key) {
            return Err(RegisterError::ConflictingInstanceFunction {
                type_info,
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| {
            let ret = f.vm_call(vm, args);
            Box::pin(async move { ret })
        });
        let signature = FnSignature::new_instance(self.path.clone(), type_info, name, Func::args());

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
    /// module.async_instance_fn("test", MyType::test)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn async_instance_fn<Func, Args>(
        &mut self,
        name: &str,
        f: Func,
    ) -> Result<(), RegisterError>
    where
        Func: AsyncInstanceFn<Args>,
    {
        let ty = Func::instance_value_type();
        let type_info = Func::instance_value_type_info();

        let key = (ty, name.to_owned());

        if self.instance_functions.contains_key(&key) {
            return Err(RegisterError::ConflictingInstanceFunction {
                type_info,
                name: name.to_owned(),
            });
        }

        let handler: Box<Handler> = Box::new(move |vm, args| f.vm_call(vm, args));
        let signature = FnSignature::new_instance(self.path.clone(), type_info, name, Func::args());

        self.instance_functions
            .insert(key.clone(), (handler, signature));
        Ok(())
    }
}

/// Trait used to provide the [global_fallible_fn][Functions::global_fallible_fn] function.
pub trait GlobalFallibleFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [global_fn][Functions::global_fn] function.
pub trait GlobalFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_fn][Self::async_fn] function.
pub trait AsyncFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Perform the vm call.
    fn vm_call<'vm>(self, vm: &'vm mut Vm, args: usize) -> BoxFuture<'vm, Result<(), VmError>>;
}

/// Trait used to provide the [instance_fallible_fn][Functions::instance_fallible_fn] function.
pub trait InstanceFallibleFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type info of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [instance_fn][Functions::instance_fn] function.
pub trait InstanceFn<Args>: 'static + Copy + Send + Sync {
    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Access the value type info of the instance.
    fn instance_value_type_info() -> ValueTypeInfo;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
}

/// Trait used to provide the [async_instance_fn][Functions::async_instance_fn] function.
pub trait AsyncInstanceFn<Args>: 'static + Copy + Send + Sync {
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

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError> {
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

            fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError> {
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
            Ok($ret) => $ret,
            Err(error) => {
                return Err(VmError::ReturnConversionError {
                    error,
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
