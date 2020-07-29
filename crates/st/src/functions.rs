use crate::collections::{hash_map, HashMap};
use crate::error;
use crate::hash::Hash;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::ValueType;
use crate::vm::{Vm, VmError};
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
    /// Tried to insert a module that conflicted with an already existing one.
    #[error("module `{name}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        name: ModuleName,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
}

/// The name of a module.
#[derive(Debug, Clone, Default)]
pub struct ModuleName {
    path: Vec<String>,
}

impl ModuleName {
    /// Construct a new module name.
    fn of<I>(iter: I) -> Self
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

    /// Return the hash of the module.
    pub fn hash(&self) -> Hash {
        Hash::module(&self.path)
    }
}

impl fmt::Display for ModuleName {
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

/// Helper alias for boxed futures.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The handler of a function.
type Handler = dyn for<'vm> Fn(&'vm mut Vm, usize) -> BoxFuture<'vm, Result<(), VmError>> + Sync;

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

/// Functions visible to the virtual machine.
#[derive(Default)]
pub struct Functions {
    /// The current global module.
    global_module: GlobalModule,
    /// Registered modules by hash.
    modules: HashMap<Hash, Module>,
}

impl Functions {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Functions::default()
    }

    /// Access the global module.
    pub fn global_module(&self) -> &GlobalModule {
        &self.global_module
    }

    /// Return a mutable variant of the global module.
    pub fn global_module_mut(&mut self) -> &mut GlobalModule {
        &mut self.global_module
    }

    /// Iterate over all modules.
    pub fn iter_modules(&self) -> impl Iterator<Item = (Hash, &Module)> {
        let mut it = self.modules.iter();

        std::iter::from_fn(move || {
            let (hash, m) = it.next()?;
            Some((*hash, m))
        })
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, RegisterError> {
        let mut functions = Self::new();
        crate::packages::core::install(&mut functions)?;
        crate::packages::bytes::install(&mut functions)?;
        crate::packages::string::install(&mut functions)?;
        Ok(functions)
    }

    /// Construct or insert functions into the module with the given name.
    pub fn module_mut<I>(&mut self, name: I) -> Result<&mut Module, RegisterError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let name = ModuleName::of(name);
        let hash = name.hash();

        match self.modules.entry(hash) {
            hash_map::Entry::Occupied(e) => Err(RegisterError::ConflictingModule {
                hash,
                name: e.get().name.clone(),
            }),
            hash_map::Entry::Vacant(e) => {
                let new_module = Module::with_name(name);
                Ok(e.insert(new_module))
            }
        }
    }

    /// Lookup module by hash.
    pub fn lookup_module(&self, module: Hash) -> Option<&Module> {
        self.modules.get(&module)
    }

    /// Lookup the given function in the global module.
    pub fn lookup(&self, hash: Hash) -> Option<&Handler> {
        let handler = self.global_module.lookup(hash)?;
        Some(&*handler)
    }
}

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct GlobalModule {
    /// Free functions.
    functions: HashMap<Hash, Box<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
}

impl GlobalModule {
    /// Construct a new global module.
    pub fn new() -> Self {
        Self::default()
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

    /// Register a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::GlobalModule::default();
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
    /// let mut module = st::GlobalModule::default();
    ///
    /// module.global_fn("bytes", StringQueue::new)?;
    /// module.instance_fn("len", StringQueue::len)?;
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
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::GlobalModule::default();
    ///
    /// module.async_fn("empty", || async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("string", |a: String| async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("optional", |a: Option<String>| async { Ok::<_, st::Error>(()) })?;
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
    /// let mut module = st::GlobalModule::default();
    ///
    /// module.instance_fallible_fn("len", |s: &str| Ok::<_, st::Error>(s.len()))?;
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
    /// let mut module = st::GlobalModule::default();
    ///
    /// module.global_fn("bytes", StringQueue::new)?;
    /// module.instance_fn("len", StringQueue::len)?;
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
    /// let mut module = st::GlobalModule::default();
    ///
    /// module.async_instance_fn("test", MyType::test)?;
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
    pub fn raw_fn<F>(&mut self, name: &str, f: F) -> Result<Hash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), VmError> + Send + Sync,
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
        O: Future<Output = Result<(), VmError>>,
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

/// A collection of functions that can be looked up by type.
#[derive(Default)]
pub struct Module {
    /// The name of the module.
    name: ModuleName,
    /// Free functions.
    functions: HashMap<Hash, Box<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
}

impl Module {
    /// Construct a new module with the given name.
    pub fn with_name(name: ModuleName) -> Self {
        Self {
            name,
            functions: Default::default(),
            functions_info: Default::default(),
        }
    }

    /// Get the module name.
    pub fn name(&self) -> &ModuleName {
        &self.name
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
    /// # fn main() -> anyhow::Result<()> {
    /// let mut module = st::Module::default();
    ///
    /// module.async_fn("empty", || async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("string", |a: String| async { Ok::<_, st::Error>(()) })?;
    /// module.async_fn("optional", |a: Option<String>| async { Ok::<_, st::Error>(()) })?;
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

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn raw_fn<F>(&mut self, name: &str, f: F) -> Result<Hash, RegisterError>
    where
        for<'vm> F: 'static + Copy + Fn(&'vm mut Vm, usize) -> Result<(), VmError> + Send + Sync,
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
        O: Future<Output = Result<(), VmError>>,
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
    /// The name of the instance for diagnostics purposes.
    fn instance_name() -> &'static str;

    /// Get the number of arguments.
    fn args() -> usize;

    /// Access the value type of the instance.
    fn instance_value_type() -> ValueType;

    /// Perform the vm call.
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
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
    fn vm_call(self, vm: &mut Vm, args: usize) -> Result<(), VmError>;
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
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
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
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
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
            fn instance_name() -> &'static str {
                std::any::type_name::<Inst>()
            }

            fn args() -> usize {
                $count
            }

            fn instance_value_type() -> ValueType {
                Inst::reflect_value_type()
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
