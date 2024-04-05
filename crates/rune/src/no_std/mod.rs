//! Public types related to using rune in #[no_std] environments.

use crate::runtime::vm_diagnostics::VmDiagnosticsObj;

/// Environment that needs to be stored somewhere.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RawEnv {
    pub(crate) context: *const (),
    pub(crate) unit: *const (),
    pub(crate) diagnostics: Option<VmDiagnosticsObj>,
}

impl RawEnv {
    /// Initialize an empty raw environment.
    pub const fn null() -> RawEnv {
        RawEnv {
            context: core::ptr::null(),
            unit: core::ptr::null(),
            diagnostics: None,
        }
    }
}

/// Defines a static budget and environment implementation suitable for
/// singlethreaded no-std environments. This can be used in `#[no_std]`
/// environments to implement the necessary hooks for Rune to work.
///
/// The alternative is to implement these manually.
///
/// If the `std` feature is enabled, thread-local budgeting will be used and
/// calling this will do nothing.
///
/// # Examples
///
/// ```
/// rune::no_std::static_env!();
/// ```
#[macro_export]
macro_rules! static_env {
    () => {
        $crate::no_std::__static_env!();
    };
}

#[cfg(feature = "std")]
#[macro_export]
#[doc(hidden)]
macro_rules! __static_env {
    () => {};
}

#[cfg(not(feature = "std"))]
#[macro_export]
#[doc(hidden)]
macro_rules! __static_env {
    () => {
        const _: () = {
            use $crate::no_std::RawEnv;

            static mut BUDGET: usize = usize::MAX;
            static mut MEMORY: usize = usize::MAX;
            static mut RAW_ENV: RawEnv = RawEnv::null();

            /// Necessary hook to abort the current process.
            #[no_mangle]
            extern "C" fn __rune_alloc_abort() -> ! {
                ::core::intrinsics::abort()
            }

            #[no_mangle]
            extern "C" fn __rune_alloc_memory_take(amount: usize) -> bool {
                unsafe {
                    if MEMORY == usize::MAX {
                        return true;
                    }

                    if MEMORY >= amount {
                        MEMORY -= amount;
                        return true;
                    }

                    return false;
                }
            }

            /// Release the given amount of memory to the current budget.
            #[no_mangle]
            extern "C" fn __rune_alloc_memory_release(amount: usize) {
                unsafe {
                    if MEMORY == usize::MAX {
                        return;
                    }

                    MEMORY = MEMORY.saturating_add(amount);
                }
            }

            /// Get the remaining memory budget for the current thread.
            #[no_mangle]
            extern "C" fn __rune_alloc_memory_get() -> usize {
                unsafe { MEMORY }
            }

            /// Replace the memory budget for the current thread and return the one which
            /// was previously set.
            #[no_mangle]
            extern "C" fn __rune_alloc_memory_replace(value: usize) -> usize {
                unsafe { core::ptr::replace(core::ptr::addr_of_mut!(MEMORY), value) }
            }

            #[no_mangle]
            extern "C" fn __rune_budget_take() -> bool {
                // SAFETY: this is only ever executed in a singlethreaded environment.
                unsafe {
                    if BUDGET == usize::MAX {
                        return true;
                    } else {
                        BUDGET = BUDGET.saturating_sub(1);
                        BUDGET != 0
                    }
                }
            }

            #[no_mangle]
            extern "C" fn __rune_budget_replace(value: usize) -> usize {
                // SAFETY: this is only ever executed in a singlethreaded environment.
                unsafe { core::ptr::replace(core::ptr::addr_of_mut!(BUDGET), value) }
            }

            #[no_mangle]
            extern "C" fn __rune_budget_get() -> usize {
                // SAFETY: this is only ever executed in a singlethreaded environment.
                unsafe { BUDGET }
            }

            #[no_mangle]
            extern "C" fn __rune_env_get() -> RawEnv {
                // SAFETY: this is only ever executed in a singlethreaded environment.
                unsafe { RAW_ENV }
            }

            #[no_mangle]
            extern "C" fn __rune_env_replace(env: RawEnv) -> RawEnv {
                // SAFETY: this is only ever executed in a singlethreaded environment.
                unsafe { core::ptr::replace(core::ptr::addr_of_mut!(RAW_ENV), env) }
            }
        };
    };
}

#[doc(hidden)]
pub use __static_env;
#[doc(inline)]
pub use static_env;
