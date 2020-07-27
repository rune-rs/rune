use crate::collections::HashMap;
use crate::hash::{FnHash, Hash};
use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ExternalTypeError, ValueTypeInfo};
use crate::vm::Vm;
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("function with hash already exists: {0:?}")]
    ConflictingFunction(FnHash),
}
#[derive(Debug, Error)]
pub enum CallError {
    #[error("stack was empty")]
    StackEmpty,
    #[error("failed to convert argument #{0} from `{1}` into `{2}`")]
    ArgumentConversionError(usize, ValueTypeInfo, &'static str),
    #[error("failed to resolve type info for external type")]
    ExternalTypeError(#[source] ExternalTypeError),
}

/// The handler of a function.
type FnHandler = dyn (for<'stack> Fn(
        &'stack mut Vm,
        usize,
    ) -> Pin<Box<dyn Future<Output = Result<(), CallError>> + 'stack>>)
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
    pub fn with_default_packages() -> Result<Self, Error> {
        let mut functions = Self::new();
        crate::packages::core::install(&mut functions)?;
        Ok(functions)
    }

    /// Lookup the given function.
    pub fn lookup(&self, hash: FnHash) -> Option<&FnHandler> {
        let handler = self.handlers.get(&hash)?;
        Some(&*handler)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn register_raw<F>(&mut self, name: &str, f: F) -> Result<FnHash, Error>
    where
        for<'stack> F: 'static + Copy + Fn(&'stack mut Vm, usize) + Send + Sync,
    {
        let hash = Hash::of(name);
        let hash = FnHash::raw(hash);

        self.handlers.insert(
            hash,
            Box::new(move |vm, args| {
                Box::pin(async move {
                    f(vm, args);
                    Ok(())
                })
            }),
        );

        Ok(hash)
    }

    /// Register a raw function which interacts directly with the virtual
    /// machine.
    pub fn register_raw_async<F, O>(&mut self, name: &str, f: F) -> Result<FnHash, Error>
    where
        for<'stack> F: 'static + Copy + Fn(&'stack mut Vm, usize) -> O + Send + Sync,
        O: Future<Output = ()>,
    {
        let hash = Hash::of(name);
        let hash = FnHash::raw(hash);

        self.handlers.insert(
            hash,
            Box::new(move |vm, args| {
                Box::pin(async move {
                    f(vm, args).await;
                    Ok(())
                })
            }),
        );

        Ok(hash)
    }
}

/// Trait used to provide the [register][Self::register] function.
pub trait Register<Func, Ret, Args> {
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
    /// functions.register("empty", || ())?;
    /// functions.register("string", |a: String| ())?;
    /// functions.register("optional", |a: Option<String>| ())?;
    /// # Ok(())
    /// # }
    /// ```
    fn register(&mut self, name: &str, f: Func) -> Result<FnHash, Error>;
}

/// Trait used to provide the [register][Self::register] function.
pub trait RegisterAsync<Func, Ret, Args> {
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
    /// functions.register_async("empty", || async { () })?;
    /// functions.register_async("string", |a: String| async { () })?;
    /// functions.register_async("optional", |a: Option<String>| async { () })?;
    /// # Ok(())
    /// # }
    /// ```
    fn register_async(&mut self, name: &str, f: Func) -> Result<FnHash, Error>;
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
        impl<Func, Ret, $($ty,)*> Register<Func, Ret, ($($ty,)*)> for Functions
        where
            Func: 'static + Copy + (Fn($($ty,)*) -> Ret) + Send + Sync,
            Ret: ToValue,
            $($ty: FromValue + ReflectValueType,)*
        {
            fn register(&mut self, name: &str, f: Func) -> Result<FnHash, Error> {
                let hash = FnHash::of(name, &[$($ty::reflect_value_type(),)*]);

                if self.handlers.contains_key(&hash) {
                    return Err(Error::ConflictingFunction(hash));
                }

                let handler: Box<FnHandler> = Box::new(move |vm, _| Box::pin(async move {
                    $(
                        let $var = match vm.managed_pop() {
                            Some(value) => match $ty::from_value(value, vm) {
                                Ok(v) => v,
                                Err(v) => {
                                    let ty = v.type_info(vm).map_err(CallError::ExternalTypeError)?;

                                    return Err(CallError::ArgumentConversionError(
                                        $count - $num,
                                        ty,
                                        type_name::<$ty>()
                                    ));
                                }
                            }
                            None => return Err(CallError::StackEmpty),
                        };
                    )*

                    let ret = f($($var,)*);
                    let ret = ret.to_value(vm).unwrap();
                    vm.managed_push(ret);
                    Ok(())
                }));

                self.handlers.insert(hash, handler);
                Ok(hash)
            }
        }

        impl<Func, Ret, $($ty,)*> RegisterAsync<Func, Ret, ($($ty,)*)> for Functions
        where
            Func: 'static + Copy + (Fn($($ty,)*) -> Ret) + Send + Sync,
            Ret: Future,
            Ret::Output: ToValue,
            $($ty: FromValue + ReflectValueType,)*
        {
            fn register_async(&mut self, name: &str, f: Func) -> Result<FnHash, Error> {
                let hash = FnHash::of(name, &[$($ty::reflect_value_type(),)*]);

                if self.handlers.contains_key(&hash) {
                    return Err(Error::ConflictingFunction(hash));
                }

                let handler: Box<FnHandler> = Box::new(move |vm, _| Box::pin(async move {
                    $(
                        let $var = match vm.managed_pop() {
                            Some(value) => match $ty::from_value(value, vm) {
                                Ok(v) => v,
                                Err(v) => {
                                    let ty = v.type_info(vm).map_err(CallError::ExternalTypeError)?;

                                    return Err(CallError::ArgumentConversionError(
                                        $count - $num,
                                        ty,
                                        type_name::<$ty>()
                                    ));
                                }
                            }
                            None => return Err(CallError::StackEmpty),
                        };
                    )*

                    let ret = f($($var,)*).await;
                    let ret = ret.to_value(vm).unwrap();
                    vm.managed_push(ret);
                    Ok(())
                }));

                self.handlers.insert(hash, handler);
                Ok(hash)
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
