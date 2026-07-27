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

        let env = unsafe {
            self::no_std::rune_env_replace(Env {
                context: Some(NonNull::new_unchecked(context.cast_mut())),
                unit: Some(NonNull::new_unchecked(unit.cast_mut())),
                globals,
                diagnostics,
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
}

impl Env {
    const fn null() -> Self {
        Self {
            context: None,
            unit: None,
            globals: None,
            diagnostics: None,
        }
    }
}
