//! Thread-local access to the current context.
//!
//! This provides access to functions to call specific protocol functions, like:
//! * [super::Value::into_iter]
//! * [super::Value::debug_fmt]
//! * [super::Value::into_type_name]
//!
//! See the corresponding function for documentation.

use core::mem::ManuallyDrop;
use core::ptr::NonNull;

#[cfg_attr(feature = "std", path = "env/std.rs")]
mod no_std;

use crate::alloc::alloc::Global;
use rust_alloc::rc::Rc;

use crate::runtime::globals::GlobalsInner;
use crate::runtime::vm_diagnostics::VmDiagnosticsObj;
use crate::runtime::{Globals, RuntimeContext, Unit, VmError, VmErrorKind};
use crate::sync::Arc;

/// Access shared parts of the environment.
///
/// This does not take ownership of the environment, so the environment can be
/// recursively accessed.
pub(crate) fn shared<F, T>(c: F) -> Result<T, VmError>
where
    F: FnOnce(&Arc<RuntimeContext>, &Arc<Unit>, &Globals) -> Result<T, VmError>,
{
    let env = self::no_std::rune_env_get();

    let Env {
        context: Some(context),
        unit: Some(unit),
        globals,
        ..
    } = env
    else {
        return Err(VmError::new(VmErrorKind::MissingInterfaceEnvironment));
    };

    // Safety: context and unit can only be registered publicly through
    // [`Guard`], which makes sure that they are live for the duration of the
    // registration.
    let context =
        unsafe { ManuallyDrop::new(Arc::from_raw_in(context.as_ptr().cast_const(), Global)) };
    let unit = unsafe { ManuallyDrop::new(Arc::from_raw_in(unit.as_ptr().cast_const(), Global)) };
    let globals = unsafe { ManuallyDrop::new(globals_from_raw(globals)) };
    c(&context, &unit, &globals)
}

/// Call the given closure with access to the checked environment accessing it
/// exclusively.
///
/// This takes ownership of the environment, so recursive calls are not
/// supported.
pub(crate) fn exclusive<F, T>(c: F) -> Result<T, VmError>
where
    F: FnOnce(
        &Arc<RuntimeContext>,
        &Arc<Unit>,
        &Globals,
        Option<&mut VmDiagnosticsObj>,
    ) -> Result<T, VmError>,
{
    let guard = Guard {
        env: self::no_std::rune_env_replace(Env::null()),
    };

    let Env {
        context: Some(context),
        unit: Some(unit),
        globals,
        ..
    } = guard.env
    else {
        return Err(VmError::new(VmErrorKind::MissingInterfaceEnvironment));
    };

    // Safety: context and unit can only be registered publicly through
    // [`Guard`], which makes sure that they are live for the duration of the
    // registration.
    let context =
        unsafe { ManuallyDrop::new(Arc::from_raw_in(context.as_ptr().cast_const(), Global)) };
    let unit = unsafe { ManuallyDrop::new(Arc::from_raw_in(unit.as_ptr().cast_const(), Global)) };
    let globals = unsafe { ManuallyDrop::new(globals_from_raw(globals)) };
    let diagnostics = match guard.env.diagnostics {
        Some(mut d) => Some(unsafe { d.as_mut() }),
        None => None,
    };

    c(&context, &unit, &globals, diagnostics)
}

/// Reconstruct a [`Globals`] handle from the raw pointer stored in the
/// environment.
///
/// # Safety
///
/// The pointer must have been produced by [`Guard::new`] and the guard which
/// produced it must still be live. The returned handle must not be dropped,
/// since it does not own the reference count it reconstructs.
unsafe fn globals_from_raw(globals: Option<NonNull<GlobalsInner>>) -> Globals {
    let Some(globals) = globals else {
        return Globals::empty();
    };

    Globals::from_inner(Some(unsafe { Rc::from_raw(globals.as_ptr().cast_const()) }))
}

pub(crate) struct Guard {
    env: Env,
}

impl Guard {
    /// Construct a new environment guard with the given context and unit.
    ///
    /// # Safety
    ///
    /// The returned guard must be dropped before the pointed to elements are.
    #[inline]
    pub(crate) fn new(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        globals: Globals,
        diagnostics: Option<NonNull<VmDiagnosticsObj>>,
    ) -> Guard {
        let (context, Global) = Arc::into_raw_with_allocator(context);
        let (unit, Global) = Arc::into_raw_with_allocator(unit);

        let globals = globals.into_inner().map(|globals| {
            let globals = Rc::into_raw(globals);
            unsafe { NonNull::new_unchecked(globals.cast_mut()) }
        });

        // The depth of the walk in progress is carried over, since registering
        // an environment is what a walk which goes through a protocol function
        // does, and the nesting has to be counted across it. The same goes for
        // how deeply executions are nested, since registering an environment is
        // also what running one does.
        let Env {
            depth, executions, ..
        } = self::no_std::rune_env_get();

        let env = unsafe {
            self::no_std::rune_env_replace(Env {
                context: Some(NonNull::new_unchecked(context.cast_mut())),
                unit: Some(NonNull::new_unchecked(unit.cast_mut())),
                globals,
                diagnostics,
                depth,
                executions,
            })
        };

        Guard { env }
    }
}

impl Drop for Guard {
    #[inline]
    fn drop(&mut self) {
        let old_env = self::no_std::rune_env_replace(self.env);

        unsafe {
            if let Some(context) = old_env.context {
                drop(Arc::from_raw_in(context.as_ptr().cast_const(), Global));
            }

            if let Some(unit) = old_env.unit {
                drop(Arc::from_raw_in(unit.as_ptr().cast_const(), Global));
            }

            if let Some(globals) = old_env.globals {
                drop(Rc::from_raw(globals.as_ptr().cast_const()));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Env {
    context: Option<NonNull<RuntimeContext>>,
    unit: Option<NonNull<Unit>>,
    globals: Option<NonNull<GlobalsInner>>,
    diagnostics: Option<NonNull<VmDiagnosticsObj>>,
    /// How deeply the walk over a value in progress has descended.
    ///
    /// See [`enter_value`].
    depth: usize,
    /// How deeply the executions in progress are nested.
    ///
    /// See [`enter_execution`].
    executions: usize,
}

impl Env {
    const fn null() -> Self {
        Self {
            context: None,
            unit: None,
            globals: None,
            diagnostics: None,
            depth: 0,
            executions: 0,
        }
    }
}

/// How deeply a walk over a value graph is allowed to nest.
///
/// Comparing, ordering, hashing and formatting a value all descend into the
/// values it is made of by recursing, and a value graph is built at runtime, so
/// how deep it is has nothing to do with how deep the source was. Comparing two
/// tuples nested somewhere between 300 and 400 deep was enough to exhaust an
/// 8 MiB stack in an unoptimised build when this was measured, so the bound is
/// well under that.
///
/// Dropping a value is bounded differently, since a destructor cannot fail. A
/// graph of values is taken apart over a worklist instead, see the `dismantle`
/// module.
pub(crate) const MAX_VALUE_DEPTH: usize = 64;

/// Enter a level of a walk over a value.
///
/// The level is left again when the returned guard is dropped, including when
/// the walk is left through an error.
///
/// The depth is kept here rather than in the caller performing the walk,
/// because a walk which goes through a protocol function implemented natively
/// gets a caller of its own, and the nesting has to be counted across it.
pub(crate) fn enter_value() -> Result<EnterValue, VmError> {
    let mut env = self::no_std::rune_env_get();

    if env.depth >= MAX_VALUE_DEPTH {
        return Err(VmError::new(VmErrorKind::MaxValueDepth {
            max: MAX_VALUE_DEPTH,
        }));
    }

    env.depth += 1;
    self::no_std::rune_env_replace(env);
    Ok(EnterValue)
}

/// The guard returned by [`enter_value`].
#[non_exhaustive]
pub(crate) struct EnterValue;

impl Drop for EnterValue {
    fn drop(&mut self) {
        let mut env = self::no_std::rune_env_get();
        env.depth = env.depth.saturating_sub(1);
        self::no_std::rune_env_replace(env);
    }
}

/// How deeply executions are allowed to nest.
///
/// A call which a native function performs - the closure handed to `sort_by`,
/// the one handed to `Option::map`, a protocol function implemented in Rune -
/// cannot be spliced into the machine which is running, since the native frame
/// in between needs the value the call produces in order to return. So it is
/// driven by a machine of its own instead, which costs a native frame for every
/// level a script nests one, and how deeply a script nests them is not bounded
/// by anything the compiler applies.
///
/// A level cost a little under 18 KiB in an unoptimised build when this was
/// measured, which exhausted the 2 MiB stack a test runs on at around 120
/// levels, so the bound is well under that.
pub(crate) const MAX_EXECUTION_DEPTH: usize = 32;

/// Enter a level of execution nesting.
///
/// The level is left again when the returned guard is dropped, including when
/// the execution is left through an error or a suspension.
///
/// The depth is kept here rather than in the execution being driven, because
/// what nests is one execution inside the native frames of another, and those
/// have no handle on each other.
pub(crate) fn enter_execution() -> Result<EnterExecution, VmError> {
    let mut env = self::no_std::rune_env_get();

    if env.executions >= MAX_EXECUTION_DEPTH {
        return Err(VmError::new(VmErrorKind::MaxExecutionDepth {
            max: MAX_EXECUTION_DEPTH,
        }));
    }

    env.executions += 1;
    self::no_std::rune_env_replace(env);
    Ok(EnterExecution)
}

/// The guard returned by [`enter_execution`].
#[non_exhaustive]
pub(crate) struct EnterExecution;

impl Drop for EnterExecution {
    fn drop(&mut self) {
        let mut env = self::no_std::rune_env_get();
        env.executions = env.executions.saturating_sub(1);
        self::no_std::rune_env_replace(env);
    }
}
