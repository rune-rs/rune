mod assert_send;
mod caller;
mod consts;
mod fixed_vec;
mod gen;

pub(crate) use self::assert_send::AssertSend;
pub(crate) use self::caller::Caller;
pub(crate) use self::consts::Consts;
pub(crate) use self::fixed_vec::{CapacityError, FixedVec};
pub(crate) use self::gen::Gen;

#[cfg(debug_assertions)]
macro_rules! _rune_diagnose {
    ($($tt:tt)*) => {
        if $crate::shared::rune_assert().is_panic() {
            panic!($($tt)*);
        } else {
            tracing::trace!($($tt)*);
        }
    };
}

#[cfg(not(debug_assertions))]
macro_rules! _rune_diagnose {
    ($($tt:tt)*) => {
        tracing::trace!($($tt)*);
    };
}

/// A macro for logging or panicking based on the current assertions model.
///
/// The assertion model can be changed from logging to panicking by setting
/// the `RUNE_ASSERT=panic` environment.
#[doc(inline)]
pub(crate) use _rune_diagnose as rune_diagnose;

#[cfg(debug_assertions)]
pub(crate) enum RuneAssert {
    /// Assert should panic.
    Panic,
    /// Assert should log an error.
    Error,
}

#[cfg(debug_assertions)]
impl RuneAssert {
    /// Test if the assert is a panic.
    pub(crate) fn is_panic(&self) -> bool {
        matches!(self, Self::Panic)
    }
}

#[cfg(all(debug_assertions, not(feature = "std")))]
mod r#impl {
    use core::fmt;

    use super::RuneAssert;

    pub(crate) struct Backtrace;

    impl Backtrace {
        #[inline]
        pub(crate) fn capture() -> Self {
            Self
        }
    }

    impl fmt::Display for Backtrace {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "backtrace not available (missing feature `std`)")
        }
    }

    #[inline]
    pub(crate) fn rune_assert() -> RuneAssert {
        RuneAssert::Error
    }
}

/// Test whether current assertions model should panic.
#[cfg(all(debug_assertions, feature = "std"))]
mod r#impl {
    use core::sync::atomic::{AtomicU8, Ordering};

    use std::env;
    use std::thread::panicking;

    pub(crate) use std::backtrace::Backtrace;

    use super::RuneAssert;

    const VAR: &str = "RUNE_ASSERT";

    static ENABLED: AtomicU8 = AtomicU8::new(0);

    pub(crate) fn rune_assert() -> RuneAssert {
        let mut value = ENABLED.load(Ordering::Relaxed);

        if value == 0 {
            value = match env::var(VAR).as_deref() {
                Ok("panic") => 1,
                _ => 2,
            };

            ENABLED.store(value, Ordering::Relaxed);
        }

        match value {
            1 if !panicking() => RuneAssert::Panic,
            _ => RuneAssert::Error,
        }
    }
}

#[cfg(debug_assertions)]
pub(crate) use self::r#impl::*;
