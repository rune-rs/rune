use core::fmt;
use core::future::Future;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::module;
use crate::runtime::{
    Args, Call, ConstValue, FromValue, FunctionHandler, OwnedTuple, Rtti, RuntimeContext, Stack,
    Unit, Value, VariantRtti, Vm, VmCall, VmErrorKind, VmHalt, VmResult,
};
use crate::shared::AssertSend;
use crate::Any;
use crate::Hash;

/// The type of a function in Rune.
///
/// Functions can be called using call expression syntax, such as `<expr>()`.
///
/// There are multiple different kind of things which can be coerced into a
/// function in Rune:
/// * Regular functions.
/// * Closures (which might or might not capture their environment).
/// * Built-in constructors for tuple types (tuple structs, tuple variants).
///
/// # Examples
///
/// ```rune
/// // Captures the constructor for the `Some(<value>)` tuple variant.
/// let build_some = Some;
/// assert_eq!(build_some(42), Some(42));
///
/// fn build(value) {
///     Some(value)
/// }
///
/// // Captures the function previously defined.
/// let build_some = build;
/// assert_eq!(build_some(42), Some(42));
/// ```
#[derive(Any)]
#[repr(transparent)]
#[rune(builtin, static_type = FUNCTION_TYPE)]
pub struct Function(FunctionImpl<Value>);

impl Function {
    /// Construct a [Function] from a Rust closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub fn main(function) {
    ///             function(41)
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    ///
    /// let function = Function::new(|value: u32| value + 1);
    ///
    /// assert_eq!(function.type_hash(), Hash::EMPTY);
    ///
    /// let value = vm.call(["main"], (function,))?;
    /// let value: u32 = rune::from_value(value)?;
    /// assert_eq!(value, 42);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// Asynchronous functions:
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// # futures_executor::block_on(async move {
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub async fn main(function) {
    ///             function(41).await
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    ///
    /// let function = Function::new(|value: u32| async move { value + 1 });
    ///
    /// assert_eq!(function.type_hash(), Hash::EMPTY);
    ///
    /// let value = vm.async_call(["main"], (function,)).await?;
    /// let value: u32 = rune::from_value(value)?;
    /// assert_eq!(value, 42);
    /// # Ok::<_, rune::support::Error>(())
    /// # })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn new<F, A, K>(f: F) -> Self
    where
        F: module::Function<A, K>,
        K: module::FunctionKind,
    {
        Self(FunctionImpl {
            inner: Inner::FnHandler(FnHandler {
                handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
                hash: Hash::EMPTY,
            }),
        })
    }

    /// See [`Function::new`].
    #[deprecated = "Use Function::new() instead"]
    pub fn function<F, A, K>(f: F) -> Self
    where
        F: module::Function<A, K>,
        K: module::FunctionKind,
    {
        Self::new(f)
    }

    /// See [`Function::function`].
    #[deprecated = "Use Function::new() instead"]
    pub fn async_function<F, A>(f: F) -> Self
    where
        F: module::Function<A, module::Async>,
    {
        Self::new(f)
    }

    /// Perform an asynchronous call over the function which also implements
    /// [Send].
    pub async fn async_send_call<A, T>(&self, args: A) -> VmResult<T>
    where
        A: Send + Args,
        T: Send + FromValue,
    {
        self.0.async_send_call(args).await
    }

    /// Perform a call over the function represented by this function pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn add(a, b) {
    ///             a + b
    ///         }
    ///
    ///         pub fn main() { add }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let value = vm.call(["main"], ())?;
    ///
    /// let value: Function = rune::from_value(value)?;
    /// assert_eq!(value.call::<_, u32>((1, 2)).into_result()?, 3);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn call<A, T>(&self, args: A) -> VmResult<T>
    where
        A: Args,
        T: FromValue,
    {
        self.0.call(args)
    }

    /// Call with the given virtual machine. This allows for certain
    /// optimizations, like avoiding the allocation of a new vm state in case
    /// the call is internal.
    ///
    /// A stop reason will be returned in case the function call results in
    /// a need to suspend the execution.
    pub(crate) fn call_with_vm(&self, vm: &mut Vm, args: usize) -> VmResult<Option<VmHalt>> {
        self.0.call_with_vm(vm, args)
    }

    /// Create a function pointer from a handler.
    pub(crate) fn from_handler(handler: Arc<FunctionHandler>, hash: Hash) -> Self {
        Self(FunctionImpl::from_handler(handler, hash))
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_vm_offset(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
        hash: Hash,
    ) -> Self {
        Self(FunctionImpl::from_offset(
            context, unit, offset, call, args, hash,
        ))
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_vm_closure(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
        environment: Box<[Value]>,
        hash: Hash,
    ) -> Self {
        Self(FunctionImpl::from_closure(
            context,
            unit,
            offset,
            call,
            args,
            environment,
            hash,
        ))
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_unit_struct(rtti: Arc<Rtti>) -> Self {
        Self(FunctionImpl::from_unit_struct(rtti))
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_tuple_struct(rtti: Arc<Rtti>, args: usize) -> Self {
        Self(FunctionImpl::from_tuple_struct(rtti, args))
    }

    /// Create a function pointer that constructs a empty variant.
    pub(crate) fn from_unit_variant(rtti: Arc<VariantRtti>) -> Self {
        Self(FunctionImpl::from_unit_variant(rtti))
    }

    /// Create a function pointer that constructs a tuple variant.
    pub(crate) fn from_tuple_variant(rtti: Arc<VariantRtti>, args: usize) -> Self {
        Self(FunctionImpl::from_tuple_variant(rtti, args))
    }

    /// Type [Hash][struct@Hash] of the underlying function.
    ///
    /// # Examples
    ///
    /// The type hash of a top-level function matches what you get out of
    /// [Hash::type_hash].
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn pony() { }
    ///
    ///         pub fn main() { pony }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let pony = vm.call(["main"], ())?;
    /// let pony: Function = rune::from_value(pony)?;
    ///
    /// assert_eq!(pony.type_hash(), Hash::type_hash(["pony"]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn type_hash(&self) -> Hash {
        self.0.type_hash()
    }

    /// Try to convert into a [SyncFunction]. This might not be possible if this
    /// function is something which is not [Sync], like a closure capturing
    /// context which is not thread-safe.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn pony() { }
    ///
    ///         pub fn main() { pony }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let pony = vm.call(["main"], ())?;
    /// let pony: Function = rune::from_value(pony)?;
    ///
    /// // This is fine, since `pony` is a free function.
    /// let pony = pony.into_sync().into_result()?;
    ///
    /// assert_eq!(pony.type_hash(), Hash::type_hash(["pony"]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// The following *does not* work, because we return a closure which tries
    /// to make use of a [Generator][crate::runtime::Generator] which is not a
    /// constant value.
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::Function;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn generator() {
    ///             yield 42;
    ///         }
    ///
    ///         pub fn main() {
    ///             let g = generator();
    ///
    ///             move || {
    ///                 g.next()
    ///             }
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let closure = vm.call(["main"], ())?;
    /// let closure: Function = rune::from_value(closure)?;
    ///
    /// // This is *not* fine since the returned closure has captured a
    /// // generator which is not a constant value.
    /// assert!(closure.into_sync().is_err());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn into_sync(self) -> VmResult<SyncFunction> {
        VmResult::Ok(SyncFunction(vm_try!(self.0.into_sync())))
    }
}

/// A callable sync function. This currently only supports a subset of values
/// that are supported by the Vm.
#[repr(transparent)]
pub struct SyncFunction(FunctionImpl<ConstValue>);

impl SyncFunction {
    /// Perform an asynchronous call over the function which also implements
    /// [Send].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::SyncFunction;
    /// use std::sync::Arc;
    ///
    /// # futures_executor::block_on(async move {
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         async fn add(a, b) {
    ///             a + b
    ///         }
    ///
    ///         pub fn main() { add }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let add = vm.call(["main"], ())?;
    /// let add: SyncFunction = rune::from_value(add)?;
    ///
    /// let value = add.async_send_call::<_, u32>((1, 2)).await.into_result()?;
    /// assert_eq!(value, 3);
    /// # Ok::<_, rune::support::Error>(())
    /// # })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub async fn async_send_call<A, T>(&self, args: A) -> VmResult<T>
    where
        A: Send + Args,
        T: Send + FromValue,
    {
        self.0.async_send_call(args).await
    }

    /// Perform a call over the function represented by this function pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::SyncFunction;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn add(a, b) {
    ///             a + b
    ///         }
    ///
    ///         pub fn main() { add }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let add = vm.call(["main"], ())?;
    /// let add: SyncFunction = rune::from_value(add)?;
    ///
    /// assert_eq!(add.call::<_, u32>((1, 2)).into_result()?, 3);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn call<A, T>(&self, args: A) -> VmResult<T>
    where
        A: Args,
        T: FromValue,
    {
        self.0.call(args)
    }

    /// Type [Hash][struct@Hash] of the underlying function.
    ///
    /// # Examples
    ///
    /// The type hash of a top-level function matches what you get out of
    /// [Hash::type_hash].
    ///
    /// ```
    /// use rune::{Hash, Vm};
    /// use rune::runtime::SyncFunction;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         fn pony() { }
    ///
    ///         pub fn main() { pony }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let pony = vm.call(["main"], ())?;
    /// let pony: SyncFunction = rune::from_value(pony)?;
    ///
    /// assert_eq!(pony.type_hash(), Hash::type_hash(["pony"]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn type_hash(&self) -> Hash {
        self.0.type_hash()
    }
}

impl TryClone for SyncFunction {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }
}

/// A stored function, of some specific kind.
struct FunctionImpl<V> {
    inner: Inner<V>,
}

impl<V> TryClone for FunctionImpl<V>
where
    V: TryClone,
{
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
        })
    }
}

impl<V> FunctionImpl<V>
where
    V: TryClone,
    OwnedTuple: TryFrom<Box<[V]>>,
    VmErrorKind: From<<OwnedTuple as TryFrom<Box<[V]>>>::Error>,
{
    fn call<A, T>(&self, args: A) -> VmResult<T>
    where
        A: Args,
        T: FromValue,
    {
        let value = match &self.inner {
            Inner::FnHandler(handler) => {
                let arg_count = args.count();
                let mut stack = vm_try!(Stack::with_capacity(arg_count));
                vm_try!(args.into_stack(&mut stack));
                vm_try!((handler.handler)(&mut stack, arg_count));
                vm_try!(stack.pop())
            }
            Inner::FnOffset(fn_offset) => vm_try!(fn_offset.call(args, ())),
            Inner::FnClosureOffset(closure) => {
                let environment = vm_try!(closure.environment.try_clone());
                let environment = vm_try!(OwnedTuple::try_from(environment));
                vm_try!(closure.fn_offset.call(args, (environment,)))
            }
            Inner::FnUnitStruct(empty) => {
                vm_try!(check_args(args.count(), 0));
                vm_try!(Value::empty_struct(empty.rtti.clone()))
            }
            Inner::FnTupleStruct(tuple) => {
                vm_try!(check_args(args.count(), tuple.args));
                vm_try!(Value::tuple_struct(
                    tuple.rtti.clone(),
                    vm_try!(args.try_into_vec())
                ))
            }
            Inner::FnUnitVariant(unit) => {
                vm_try!(check_args(args.count(), 0));
                vm_try!(Value::unit_variant(unit.rtti.clone()))
            }
            Inner::FnTupleVariant(tuple) => {
                vm_try!(check_args(args.count(), tuple.args));
                vm_try!(Value::tuple_variant(
                    tuple.rtti.clone(),
                    vm_try!(args.try_into_vec())
                ))
            }
        };

        T::from_value(value)
    }

    fn async_send_call<'a, A, T>(&'a self, args: A) -> impl Future<Output = VmResult<T>> + Send + 'a
    where
        A: 'a + Send + Args,
        T: 'a + Send + FromValue,
    {
        let future = async move {
            let value = vm_try!(self.call(args));

            let value = match value {
                Value::Future(future) => {
                    let future = vm_try!(future.take());
                    vm_try!(future.await)
                }
                other => other,
            };

            T::from_value(value)
        };

        // Safety: Future is send because there is no way to call this
        // function in a manner which allows any values from the future
        // to escape outside of this future, hence it can only be
        // scheduled by one thread at a time.
        unsafe { AssertSend::new(future) }
    }

    /// Call with the given virtual machine. This allows for certain
    /// optimizations, like avoiding the allocation of a new vm state in case
    /// the call is internal.
    ///
    /// A stop reason will be returned in case the function call results in
    /// a need to suspend the execution.
    pub(crate) fn call_with_vm(&self, vm: &mut Vm, args: usize) -> VmResult<Option<VmHalt>> {
        let reason = match &self.inner {
            Inner::FnHandler(handler) => {
                vm_try!((handler.handler)(vm.stack_mut(), args));
                None
            }
            Inner::FnOffset(fn_offset) => {
                if let Some(vm_call) = vm_try!(fn_offset.call_with_vm(vm, args, ())) {
                    return VmResult::Ok(Some(VmHalt::VmCall(vm_call)));
                }

                None
            }
            Inner::FnClosureOffset(closure) => {
                let environment = vm_try!(closure.environment.try_clone());
                let environment = vm_try!(OwnedTuple::try_from(environment));

                if let Some(vm_call) =
                    vm_try!(closure.fn_offset.call_with_vm(vm, args, (environment,)))
                {
                    return VmResult::Ok(Some(VmHalt::VmCall(vm_call)));
                }

                None
            }
            Inner::FnUnitStruct(empty) => {
                vm_try!(check_args(args, 0));
                let value = vm_try!(Value::empty_struct(empty.rtti.clone()));
                vm_try!(vm.stack_mut().push(value));
                None
            }
            Inner::FnTupleStruct(tuple) => {
                vm_try!(check_args(args, tuple.args));

                let value = vm_try!(Value::tuple_struct(
                    tuple.rtti.clone(),
                    vm_try!(vm_try!(vm.stack_mut().pop_sequence(args))),
                ));

                vm_try!(vm.stack_mut().push(value));
                None
            }
            Inner::FnUnitVariant(tuple) => {
                vm_try!(check_args(args, 0));

                let value = vm_try!(Value::unit_variant(tuple.rtti.clone()));

                vm_try!(vm.stack_mut().push(value));
                None
            }
            Inner::FnTupleVariant(tuple) => {
                vm_try!(check_args(args, tuple.args));

                let value = vm_try!(Value::tuple_variant(
                    tuple.rtti.clone(),
                    vm_try!(vm_try!(vm.stack_mut().pop_sequence(args))),
                ));

                vm_try!(vm.stack_mut().push(value));
                None
            }
        };

        VmResult::Ok(reason)
    }

    /// Create a function pointer from a handler.
    pub(crate) fn from_handler(handler: Arc<FunctionHandler>, hash: Hash) -> Self {
        Self {
            inner: Inner::FnHandler(FnHandler { handler, hash }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_offset(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
        hash: Hash,
    ) -> Self {
        Self {
            inner: Inner::FnOffset(FnOffset {
                context,
                unit,
                offset,
                call,
                args,
                hash,
            }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_closure(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
        environment: Box<[V]>,
        hash: Hash,
    ) -> Self {
        Self {
            inner: Inner::FnClosureOffset(FnClosureOffset {
                fn_offset: FnOffset {
                    context,
                    unit,
                    offset,
                    call,
                    args,
                    hash,
                },
                environment,
            }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_unit_struct(rtti: Arc<Rtti>) -> Self {
        Self {
            inner: Inner::FnUnitStruct(FnUnitStruct { rtti }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_tuple_struct(rtti: Arc<Rtti>, args: usize) -> Self {
        Self {
            inner: Inner::FnTupleStruct(FnTupleStruct { rtti, args }),
        }
    }

    /// Create a function pointer that constructs a empty variant.
    pub(crate) fn from_unit_variant(rtti: Arc<VariantRtti>) -> Self {
        Self {
            inner: Inner::FnUnitVariant(FnUnitVariant { rtti }),
        }
    }

    /// Create a function pointer that constructs a tuple variant.
    pub(crate) fn from_tuple_variant(rtti: Arc<VariantRtti>, args: usize) -> Self {
        Self {
            inner: Inner::FnTupleVariant(FnTupleVariant { rtti, args }),
        }
    }

    #[inline]
    fn type_hash(&self) -> Hash {
        match &self.inner {
            Inner::FnHandler(FnHandler { hash, .. }) | Inner::FnOffset(FnOffset { hash, .. }) => {
                *hash
            }
            Inner::FnClosureOffset(fco) => fco.fn_offset.hash,
            Inner::FnUnitStruct(func) => func.rtti.hash,
            Inner::FnTupleStruct(func) => func.rtti.hash,
            Inner::FnUnitVariant(func) => func.rtti.hash,
            Inner::FnTupleVariant(func) => func.rtti.hash,
        }
    }
}

impl FunctionImpl<Value> {
    /// Try to convert into a [SyncFunction].
    fn into_sync(self) -> VmResult<FunctionImpl<ConstValue>> {
        let inner = match self.inner {
            Inner::FnClosureOffset(closure) => {
                let mut env = vm_try!(Vec::try_with_capacity(closure.environment.len()));

                for value in Vec::from(closure.environment) {
                    vm_try!(env.try_push(vm_try!(FromValue::from_value(value))));
                }

                Inner::FnClosureOffset(FnClosureOffset {
                    fn_offset: closure.fn_offset,
                    environment: vm_try!(env.try_into_boxed_slice()),
                })
            }
            Inner::FnHandler(inner) => Inner::FnHandler(inner),
            Inner::FnOffset(inner) => Inner::FnOffset(inner),
            Inner::FnUnitStruct(inner) => Inner::FnUnitStruct(inner),
            Inner::FnTupleStruct(inner) => Inner::FnTupleStruct(inner),
            Inner::FnUnitVariant(inner) => Inner::FnUnitVariant(inner),
            Inner::FnTupleVariant(inner) => Inner::FnTupleVariant(inner),
        };

        VmResult::Ok(FunctionImpl { inner })
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.inner {
            Inner::FnHandler(handler) => {
                write!(f, "native function ({:p})", handler.handler.as_ref())?;
            }
            Inner::FnOffset(offset) => {
                write!(f, "dynamic function (at: 0x{:x})", offset.offset)?;
            }
            Inner::FnClosureOffset(closure) => {
                write!(
                    f,
                    "closure (at: 0x{:x}, env:{:?})",
                    closure.fn_offset.offset, closure.environment
                )?;
            }
            Inner::FnUnitStruct(empty) => {
                write!(f, "empty {}", empty.rtti.item)?;
            }
            Inner::FnTupleStruct(tuple) => {
                write!(f, "tuple {}", tuple.rtti.item)?;
            }
            Inner::FnUnitVariant(empty) => {
                write!(f, "variant empty {}", empty.rtti.item)?;
            }
            Inner::FnTupleVariant(tuple) => {
                write!(f, "variant tuple {}", tuple.rtti.item)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum Inner<V> {
    /// A native function handler.
    /// This is wrapped as an `Arc<dyn FunctionHandler>`.
    FnHandler(FnHandler),
    /// The offset to a free function.
    ///
    /// This also captures the context and unit it belongs to allow for external
    /// calls.
    FnOffset(FnOffset),
    /// A closure with a captured environment.
    ///
    /// This also captures the context and unit it belongs to allow for external
    /// calls.
    FnClosureOffset(FnClosureOffset<V>),
    /// Constructor for a unit struct.
    FnUnitStruct(FnUnitStruct),
    /// Constructor for a tuple.
    FnTupleStruct(FnTupleStruct),
    /// Constructor for an empty variant.
    FnUnitVariant(FnUnitVariant),
    /// Constructor for a tuple variant.
    FnTupleVariant(FnTupleVariant),
}

impl<V> TryClone for Inner<V>
where
    V: TryClone,
{
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(match self {
            Inner::FnHandler(inner) => Inner::FnHandler(inner.clone()),
            Inner::FnOffset(inner) => Inner::FnOffset(inner.clone()),
            Inner::FnClosureOffset(inner) => Inner::FnClosureOffset(inner.try_clone()?),
            Inner::FnUnitStruct(inner) => Inner::FnUnitStruct(inner.clone()),
            Inner::FnTupleStruct(inner) => Inner::FnTupleStruct(inner.clone()),
            Inner::FnUnitVariant(inner) => Inner::FnUnitVariant(inner.clone()),
            Inner::FnTupleVariant(inner) => Inner::FnTupleVariant(inner.clone()),
        })
    }
}

#[derive(Clone)]
struct FnHandler {
    /// The function handler.
    handler: Arc<FunctionHandler>,
    /// Hash for the function type
    hash: Hash,
}

impl fmt::Debug for FnHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FnHandler")
    }
}

#[derive(Clone)]
struct FnOffset {
    context: Arc<RuntimeContext>,
    /// The unit where the function resides.
    unit: Arc<Unit>,
    /// The offset of the function.
    offset: usize,
    /// The calling convention.
    call: Call,
    /// The number of arguments the function takes.
    args: usize,
    /// Hash for the function type
    hash: Hash,
}

impl FnOffset {
    /// Perform a call into the specified offset and return the produced value.
    fn call<A, E>(&self, args: A, extra: E) -> VmResult<Value>
    where
        A: Args,
        E: Args,
    {
        vm_try!(check_args(args.count(), self.args));

        let mut vm = Vm::new(self.context.clone(), self.unit.clone());

        vm.set_ip(self.offset);
        vm_try!(args.into_stack(vm.stack_mut()));
        vm_try!(extra.into_stack(vm.stack_mut()));

        self.call.call_with_vm(vm)
    }

    /// Perform a potentially optimized call into the specified vm.
    ///
    /// This will cause a halt in case the vm being called into isn't the same
    /// as the context and unit of the function.
    #[tracing::instrument(skip_all, fields(args, extra = extra.count(), ?self.offset, ?self.call, ?self.args, ?self.hash))]
    fn call_with_vm<E>(&self, vm: &mut Vm, args: usize, extra: E) -> VmResult<Option<VmCall>>
    where
        E: Args,
    {
        tracing::trace!("calling");

        vm_try!(check_args(args, self.args));

        let same_unit = matches!(self.call, Call::Immediate if vm.is_same_unit(&self.unit));
        let same_context =
            matches!(self.call, Call::Immediate if vm.is_same_context(&self.context));

        vm_try!(vm.push_call_frame(self.offset, args, !same_context));
        vm_try!(extra.into_stack(vm.stack_mut()));

        // Fast path, just allocate a call frame and keep running.
        if same_context && same_unit {
            tracing::trace!("same context and unit");
            return VmResult::Ok(None);
        }

        VmResult::Ok(Some(VmCall::new(
            self.call,
            (!same_context).then(|| self.context.clone()),
            (!same_unit).then(|| self.unit.clone()),
        )))
    }
}

impl fmt::Debug for FnOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnOffset")
            .field("context", &(&self.context as *const _))
            .field("unit", &(&self.unit as *const _))
            .field("offset", &self.offset)
            .field("call", &self.call)
            .field("args", &self.args)
            .finish()
    }
}

#[derive(Debug)]
struct FnClosureOffset<V> {
    /// The offset in the associated unit that the function lives.
    fn_offset: FnOffset,
    /// Captured environment.
    environment: Box<[V]>,
}

impl<V> TryClone for FnClosureOffset<V>
where
    V: TryClone,
{
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            fn_offset: self.fn_offset.clone(),
            environment: self.environment.try_clone()?,
        })
    }
}

#[derive(Debug, Clone)]
struct FnUnitStruct {
    /// The type of the empty.
    rtti: Arc<Rtti>,
}

#[derive(Debug, Clone)]
struct FnTupleStruct {
    /// The type of the tuple.
    rtti: Arc<Rtti>,
    /// The number of arguments the tuple takes.
    args: usize,
}

#[derive(Debug, Clone)]
struct FnUnitVariant {
    /// Runtime information fo variant.
    rtti: Arc<VariantRtti>,
}

#[derive(Debug, Clone)]
struct FnTupleVariant {
    /// Runtime information fo variant.
    rtti: Arc<VariantRtti>,
    /// The number of arguments the tuple takes.
    args: usize,
}

impl FromValue for SyncFunction {
    fn from_value(value: Value) -> VmResult<Self> {
        let function = vm_try!(value.into_function());
        let function = vm_try!(function.take());
        function.into_sync()
    }
}

from_value!(Function, into_function);

fn check_args(actual: usize, expected: usize) -> VmResult<()> {
    if actual != expected {
        return VmResult::err(VmErrorKind::BadArgumentCount { expected, actual });
    }

    VmResult::Ok(())
}

#[cfg(test)]
mod tests {
    use super::SyncFunction;

    fn assert_send<T>()
    where
        T: Send,
    {
    }

    fn assert_sync<T>()
    where
        T: Sync,
    {
    }

    #[test]
    fn assert_send_sync() {
        assert_send::<SyncFunction>();
        assert_sync::<SyncFunction>();
    }
}
