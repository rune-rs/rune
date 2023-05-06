//! Types used for defining native modules.
//!
//! A native module is one that provides rune with functions and types through
//! native Rust-based code.

mod function_meta;
pub(crate) mod module;

use core::fmt;
use core::future;

use crate::no_std::collections::{hash_map, HashMap, HashSet};
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{self, ContextError, Docs, IntoComponent, ItemBuf, Named};
use crate::macros::{MacroContext, TokenStream};
use crate::runtime::{
    ConstValue, FromValue, FullTypeOf, FunctionHandler, Future, GeneratorState, MacroHandler,
    MaybeTypeOf, Protocol, Stack, StaticType, ToValue, TypeCheck, TypeInfo, TypeOf,
    UnsafeFromValue, Value, VmErrorKind, VmResult,
};
use crate::Hash;

pub(crate) use self::function_meta::{
    AssociatedFunctionData, AssociatedFunctionKind, AssociatedFunctionName, FunctionData,
    IterFunctionArgs, ToFieldFunction, ToInstance,
};
use self::function_meta::{FunctionMeta, MacroMeta};
#[doc(hidden)]
pub use self::function_meta::{FunctionMetaData, FunctionMetaKind, MacroMetaData, MacroMetaKind};
pub(crate) use self::module::Module;

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
#[doc(hidden)]
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

/// A key that identifies an associated function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) struct AssociatedFunctionKey {
    /// The type the associated function belongs to.
    pub(crate) type_hash: Hash,
    /// The kind of the associated function.
    pub(crate) kind: AssociatedFunctionKind,
    /// The type parameters of the associated function.
    pub(crate) parameters: Hash,
}

/// The kind of a module function.
#[derive(Clone)]
pub(crate) enum ModuleFunctionKind {
    Function,
    Assoc {
        /// Type information of the associated function.
        type_info: TypeInfo,
        /// The full name of the associated function.
        name: AssociatedFunctionName,
    },
}

/// Handle to a function inserted into a module.
///
/// This is returned by methods which insert any kind of function, such as:
/// * [`Module::function_meta`].
/// * [`Module::raw_fn`].
/// * [`Module::function`].
/// * [`Module::inst_fn`].
pub struct ModuleFunction {
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) is_async: bool,
    pub(crate) args: Option<usize>,
    pub(crate) return_type: Option<FullTypeOf>,
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
    pub(crate) docs: Docs,
    pub(crate) kind: ModuleFunctionKind,
}

impl ModuleFunction {
    // Private clone implementation.
    pub(crate) fn clone_it(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            is_async: self.is_async.clone(),
            args: self.args.clone(),
            return_type: self.return_type.clone(),
            argument_types: self.argument_types.clone(),
            docs: self.docs.clone(),
            kind: self.kind.clone(),
        }
    }

    /// Set documentation for an inserted function.
    pub fn docs<I>(&mut self, docs: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }
}

/// Handle to a macro inserted into a module.
pub struct ModuleMacro {
    pub(crate) handler: Arc<MacroHandler>,
    pub(crate) docs: Docs,
}

impl ModuleMacro {
    /// Set documentation for an inserted macro.
    pub fn docs<I>(&mut self, docs: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }
}

/// Trait used to provide the [function][Module::function] function.
pub trait Function<Args>: 'static + Send + Sync {
    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [async_function][Module::async_function] function.
pub trait AsyncFunction<Args>: 'static + Send + Sync {
    /// The return type of the function.
    #[doc(hidden)]
    type Return: future::Future<Output = Self::Output>;

    /// The output produces by the future.
    #[doc(hidden)]
    type Output;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [inst_fn][Module::inst_fn] function.
pub trait InstFn<Args>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Instance;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    #[doc(hidden)]
    fn ty() -> AssocType;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [async_inst_fn][Module::async_inst_fn] function.
pub trait AsyncInstFn<Args>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Instance;

    /// The return type of the function.
    #[doc(hidden)]
    type Return: future::Future<Output = Self::Output>;

    /// The output value of the async function.
    #[doc(hidden)]
    type Output;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    #[doc(hidden)]
    fn ty() -> AssocType;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

macro_rules! impl_register {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
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

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                impl_register!(@check-args $count, args);

                #[allow(unused_mut)]
                let mut it = vm_try!(stack.drain($count));
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `stack`
                // when we return below.
                #[allow(unused)]
                let ret = unsafe {
                    impl_register!(@unsafe-vars $count, $($ty, $var, $num,)*);
                    let ret = self($(<$ty>::unsafe_coerce($var.0),)*);
                    impl_register!(@drop-stack-guards $($var),*);
                    ret
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
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
            type Output = Return::Output;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                impl_register!(@check-args $count, args);

                #[allow(unused_mut)]
                let mut it = vm_try!(stack.drain($count));
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    impl_register!(@unsafe-vars $count, $($ty, $var, $num,)*);

                    let fut = self($(<$ty>::unsafe_coerce($var.0),)*);

                    Future::new(async move {
                        let output = fut.await;
                        impl_register!(@drop-stack-guards $($var),*);
                        let value = vm_try!(output.to_value());
                        VmResult::Ok(value)
                    })
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
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

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                impl_register!(@check-args ($count + 1), args);

                #[allow(unused_mut)]
                let mut it = vm_try!(stack.drain($count + 1));
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
                    impl_register!(@unsafe-inst-vars inst, $count, $($ty, $var, $num,)*);
                    let ret = self(Instance::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);
                    impl_register!(@drop-stack-guards inst, $($var),*);
                    ret
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
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
            type Output = Return::Output;

            fn args() -> usize {
                $count + 1
            }

            fn ty() -> AssocType {
                AssocType {
                    hash: Instance::type_hash(),
                    type_info: Instance::type_info(),
                }
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                impl_register!(@check-args ($count + 1), args);

                #[allow(unused_mut)]
                let mut it = vm_try!(stack.drain($count + 1));
                let inst = it.next().unwrap();
                $(let $var = it.next().unwrap();)*
                drop(it);

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused)]
                let ret = unsafe {
                    impl_register!(@unsafe-inst-vars inst, $count, $($ty, $var, $num,)*);

                    let fut = self(Instance::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);

                    Future::new(async move {
                        let output = fut.await;
                        impl_register!(@drop-stack-guards inst, $($var),*);
                        let value = vm_try!(output.to_value());
                        VmResult::Ok(value)
                    })
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }
    };

    // Expand to function variable bindings.
    (@unsafe-vars $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: $count - $num,
            }));
        )*
    };

    // Expand to instance variable bindings.
    (@unsafe-inst-vars $inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = vm_try!(Instance::from_value($inst).with_error(|| VmErrorKind::BadArgument {
            arg: 0,
        }));

        $(
            let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: 1 + $count - $num,
            }));
        )*
    };

    // Helper variation to drop all stack guards associated with the specified variables.
    (@drop-stack-guards $($var:ident),* $(,)?) => {{
        $(drop(($var.1));)*
    }};

    (@check-args $expected:expr, $actual:expr) => {
        if $actual != $expected {
            return VmResult::err(VmErrorKind::BadArgumentCount {
                actual: $actual,
                expected: $expected,
            });
        }
    };
}

repeat_macro!(impl_register);
