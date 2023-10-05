//! Test a bug where the contract for `UnsafeFromValue` was not being properly
//! upheld by internal module helpers when registering a native function.
//!
//! This ensures that the contract "works" by checking that a value which is
//! being used hasn't had its guard dropped through a shared reference-counted
//! cell.
//!
//! See: https://github.com/rune-rs/rune/issues/344

prelude!();

use std::cell::Cell;
use std::rc::Rc;

#[test]
fn bug_344_function() -> Result<()> {
    let mut context = Context::new();
    let mut module = Module::new();

    module.function("function", function).build()?;

    context.install(module)?;
    let runtime = context.runtime()?;

    let hash = Hash::type_hash(["function"]);

    let function = runtime.function(hash).expect("expect function");

    let mut stack = Stack::new();
    stack.push(rune::to_value(GuardCheck::new())?)?;
    function(&mut stack, 1).into_result()?;
    assert_eq!(stack.pop()?.into_integer().into_result()?, 42);
    return Ok(());

    fn function(check: &GuardCheck) -> i64 {
        check.ensure_not_dropped("immediate argument");
        42
    }
}

#[test]
fn bug_344_inst_fn() -> Result<()> {
    #[rune::function(instance)]
    fn function(s: &GuardCheck, check: &GuardCheck) -> i64 {
        s.ensure_not_dropped("async self argument");
        check.ensure_not_dropped("async instance argument");
        42
    }

    let mut context = Context::new();
    let mut module = Module::new();

    module.ty::<GuardCheck>()?;
    module.function_meta(function)?;

    context.install(module)?;
    let runtime = context.runtime()?;

    let hash = Hash::associated_function(<GuardCheck as Any>::type_hash(), "function");

    let function = runtime.function(hash).expect("expect function");

    let mut stack = Stack::new();
    stack.push(rune::to_value(GuardCheck::new())?)?;
    stack.push(rune::to_value(GuardCheck::new())?)?;
    function(&mut stack, 2).into_result()?;

    assert_eq!(stack.pop()?.into_integer().into_result()?, 42);
    Ok(())
}

#[test]
fn bug_344_async_function() -> Result<()> {
    let mut context = Context::new();
    let mut module = Module::new();

    module.function("function", function).build()?;

    context.install(module)?;
    let runtime = context.runtime()?;

    let hash = Hash::type_hash(["function"]);

    let function = runtime.function(hash).expect("expect function");

    let mut stack = Stack::new();
    stack.push(rune::to_value(GuardCheck::new())?)?;
    function(&mut stack, 1).into_result()?;
    let future = stack.pop()?.into_future().into_result()?.take()?;
    assert_eq!(
        block_on(future)
            .into_result()?
            .into_integer()
            .into_result()?,
        42
    );
    return Ok(());

    async fn function(check: Shared<AnyObj>) -> i64 {
        check
            .downcast_borrow_ref::<GuardCheck>()
            .unwrap()
            .ensure_not_dropped("async argument");
        42
    }
}

#[test]
fn bug_344_async_inst_fn() -> Result<()> {
    #[rune::function(instance)]
    async fn function(s: Ref<GuardCheck>, check: Ref<GuardCheck>) -> VmResult<i64> {
        s.ensure_not_dropped("self argument");
        check.ensure_not_dropped("instance argument");
        VmResult::Ok(42)
    }

    let mut context = Context::new();
    let mut module = Module::new();

    module.ty::<GuardCheck>()?;
    module.function_meta(function)?;

    context.install(module)?;
    let runtime = context.runtime()?;

    let hash = Hash::associated_function(<GuardCheck as Any>::type_hash(), "function");

    let function = runtime.function(hash).expect("expect function");

    let mut stack = Stack::new();
    stack.push(rune::to_value(GuardCheck::new())?)?;
    stack.push(rune::to_value(GuardCheck::new())?)?;
    function(&mut stack, 2).into_result()?;

    let future = stack.pop()?.into_future().into_result()?.take()?;
    assert_eq!(
        block_on(future)
            .into_result()?
            .into_integer()
            .into_result()?,
        42
    );

    Ok(())
}

struct Guard {
    _guard: RawRef,
    dropped: Rc<Cell<bool>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}

struct GuardCheck {
    dropped: Rc<Cell<bool>>,
}

impl GuardCheck {
    fn new() -> Self {
        Self {
            dropped: Rc::new(Cell::new(false)),
        }
    }

    fn ensure_not_dropped(&self, what: &str) {
        assert!(
            !self.dropped.get(),
            "value has was previously dropped: {}",
            what
        );
    }
}

impl Any for GuardCheck {
    fn type_hash() -> Hash {
        rune_macros::hash!(GuardCheck)
    }
}

impl Named for GuardCheck {
    const BASE_NAME: RawStr = RawStr::from_str("GuardCheck");
}

impl TypeOf for GuardCheck {
    #[inline]
    fn type_hash() -> Hash {
        <Self as Any>::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        TypeInfo::Any(AnyTypeInfo::__private_new(
            <Self as Named>::BASE_NAME,
            <Self as TypeOf>::type_hash(),
        ))
    }
}

impl MaybeTypeOf for GuardCheck {
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        Some(Self::type_of())
    }
}

impl InstallWith for GuardCheck {}

impl UnsafeToRef for GuardCheck {
    type Guard = Guard;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let (output, guard) = vm_try!(value.into_any_ptr::<GuardCheck>());

        let guard = Guard {
            _guard: guard,
            // Regardless of what happens, the value is available here and the
            // refcounted value will be available even if the underlying value
            // *is* dropped prematurely because it's been cloned.
            dropped: output.as_ref().dropped.clone(),
        };

        VmResult::Ok((output.as_ref(), guard))
    }
}
