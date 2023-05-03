use super::Env;

use crate::no_std::RawEnv;

// In no-std environments, the implementor must define these functions.
//
// Normally these make use of thread-local storage, but if you want them to be
// completed disabled simply return dummy values or store it in static storage
// (if singlethreaded).
extern "C" {
    /// Get the last environment set, returning all zeros if none is set.
    pub fn __rune_env_get() -> RawEnv;

    /// Set the virtual machine instance to use and return the old value that
    /// was set.
    pub fn __rune_env_replace(vm: RawEnv) -> RawEnv;
}

pub(super) fn rune_env_get() -> Env {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { from_raw_env(__rune_env_get()) }
}

pub(super) fn rune_env_replace(env: Env) -> Env {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { from_raw_env(__rune_env_replace(from_env(env))) }
}

unsafe fn from_env(env: Env) -> RawEnv {
    RawEnv {
        context: env.context as *const _,
        unit: env.unit as *const _,
    }
}

unsafe fn from_raw_env(env: RawEnv) -> Env {
    Env {
        context: env.context as *const _,
        unit: env.unit as *const _,
    }
}
