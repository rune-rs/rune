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

use rust_alloc::sync::Arc;

use crate::runtime::vm_diagnostics::VmDiagnosticsObj;
use crate::runtime::{RuntimeContext, Unit, VmErrorKind, VmResult};

/// Access shared parts of the environment.
///
/// This does not take ownership of the environment, so the environment can be
/// recursively accessed.
pub(crate) fn shared<F, T>(c: F) -> VmResult<T>
where
    F: FnOnce(&Arc<RuntimeContext>, &Arc<Unit>) -> VmResult<T>,
{
    let env = self::no_std::rune_env_get();

    let Env {
        context: Some(context),
        unit: Some(unit),
        ..
    } = env
    else {
        return VmResult::err(VmErrorKind::MissingInterfaceEnvironment);
    };

    // Safety: context and unit can only be registered publicly through
    // [`Guard`], which makes sure that they are live for the duration of the
    // registration.
    let context = unsafe { ManuallyDrop::new(Arc::from_raw(context.as_ptr().cast_const())) };
    let unit = unsafe { ManuallyDrop::new(Arc::from_raw(unit.as_ptr().cast_const())) };
    c(&context, &unit)
}

/// Call the given closure with access to the checked environment accessing it
/// exclusively.
///
/// This takes ownership of the environment, so recursive calls are not
/// supported.
pub(crate) fn exclusive<F, T>(c: F) -> VmResult<T>
where
    F: FnOnce(&Arc<RuntimeContext>, &Arc<Unit>, Option<&mut VmDiagnosticsObj>) -> VmResult<T>,
{
    let guard = Guard {
        env: self::no_std::rune_env_replace(Env::null()),
    };

    let Env {
        context: Some(context),
        unit: Some(unit),
        ..
    } = guard.env
    else {
        return VmResult::err(VmErrorKind::MissingInterfaceEnvironment);
    };

    // Safety: context and unit can only be registered publicly through
    // [`Guard`], which makes sure that they are live for the duration of the
    // registration.
    let context = unsafe { ManuallyDrop::new(Arc::from_raw(context.as_ptr().cast_const())) };
    let unit = unsafe { ManuallyDrop::new(Arc::from_raw(unit.as_ptr().cast_const())) };
    let diagnostics = match guard.env.diagnostics {
        Some(mut d) => Some(unsafe { d.as_mut() }),
        None => None,
    };

    c(&context, &unit, diagnostics)
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
    pub(crate) fn new(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        diagnostics: Option<NonNull<VmDiagnosticsObj>>,
    ) -> Guard {
        let env = unsafe {
            self::no_std::rune_env_replace(Env {
                context: Some(NonNull::new_unchecked(Arc::into_raw(context).cast_mut())),
                unit: Some(NonNull::new_unchecked(Arc::into_raw(unit).cast_mut())),
                diagnostics,
            })
        };

        Guard { env }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let old_env = self::no_std::rune_env_replace(self.env);

        unsafe {
            if let Some(context) = old_env.context {
                drop(Arc::from_raw(context.as_ptr().cast_const()));
            }

            if let Some(unit) = old_env.unit {
                drop(Arc::from_raw(unit.as_ptr().cast_const()));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Env {
    context: Option<NonNull<RuntimeContext>>,
    unit: Option<NonNull<Unit>>,
    diagnostics: Option<NonNull<VmDiagnosticsObj>>,
}

impl Env {
    const fn null() -> Self {
        Self {
            context: None,
            unit: None,
            diagnostics: None,
        }
    }
}
