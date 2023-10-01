//! Thread-local access to the current context.
//!
//! This provides access to functions to call specific protocol functions, like:
//! * [super::Value::into_iter]
//! * [super::Value::string_debug]
//! * [super::Value::into_type_name]
//!
//! See the corresponding function for documentation.

#[cfg_attr(feature = "std", path = "env/std.rs")]
mod no_std;

use ::rust_alloc::sync::Arc;

use crate::runtime::{RuntimeContext, Unit, VmErrorKind, VmResult};

/// Call the given closure with access to the checked environment.
pub(crate) fn with<F, T>(c: F) -> VmResult<T>
where
    F: FnOnce(&Arc<RuntimeContext>, &Arc<Unit>) -> VmResult<T>,
{
    let env = self::no_std::rune_env_get();
    let Env { context, unit } = env;

    if context.is_null() || unit.is_null() {
        return VmResult::err(VmErrorKind::MissingInterfaceEnvironment);
    }

    // Safety: context and unit can only be registered publicly through
    // [Guard], which makes sure that they are live for the duration of
    // the registration.
    c(unsafe { &*context }, unsafe { &*unit })
}

pub(crate) struct Guard {
    old: Env,
}

impl Guard {
    /// Construct a new environment guard with the given context and unit.
    ///
    /// # Safety
    ///
    /// The returned guard must be dropped before the pointed to elements are.
    pub(crate) fn new(context: *const Arc<RuntimeContext>, unit: *const Arc<Unit>) -> Guard {
        let old = self::no_std::rune_env_replace(Env { context, unit });
        Guard { old }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self::no_std::rune_env_replace(self.old);
    }
}

#[derive(Debug, Clone, Copy)]
struct Env {
    context: *const Arc<RuntimeContext>,
    unit: *const Arc<Unit>,
}

impl Env {
    #[cfg(feature = "std")]
    const fn null() -> Self {
        Self {
            context: core::ptr::null(),
            unit: core::ptr::null(),
        }
    }
}
