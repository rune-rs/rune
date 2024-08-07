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

/// Test whether current assertions model should panic.
#[cfg(all(debug_assertions, feature = "std"))]
mod r#impl {
    #[cfg(feature = "fmt")]
    use core::fmt::{self, Write};
    use core::sync::atomic::{AtomicU8, Ordering};

    use std::env;
    use std::thread::panicking;

    pub(crate) use std::backtrace::Backtrace;

    pub(crate) enum RuneAssert {
        /// Assert should panic.
        Panic,
        /// Assert should trace.
        Trace,
        /// Assert should log an error.
        Error,
    }

    impl RuneAssert {
        /// Test if the assert is a panic.
        #[allow(unused)]
        pub(crate) fn is_panic(&self) -> bool {
            matches!(self, Self::Panic)
        }

        /// Test if the assert is a trace.
        #[allow(unused)]
        pub(crate) fn is_trace(&self) -> bool {
            matches!(self, Self::Trace)
        }
    }

    const VAR: &str = "RUNE_ASSERT";

    static ENABLED: AtomicU8 = AtomicU8::new(0);

    pub(crate) fn rune_assert() -> RuneAssert {
        let mut value = ENABLED.load(Ordering::Relaxed);

        if value == 0 {
            value = match env::var(VAR).as_deref() {
                Ok("panic") => 1,
                Ok("trace") => 2,
                _ => 3,
            };

            ENABLED.store(value, Ordering::Relaxed);
        }

        match value {
            1 if !panicking() => RuneAssert::Panic,
            2 if !panicking() => RuneAssert::Trace,
            _ => RuneAssert::Error,
        }
    }

    #[cfg(feature = "fmt")]
    pub(crate) struct CaptureAt {
        at: &'static str,
        done: usize,
        string: rust_alloc::string::String,
    }

    #[cfg(feature = "fmt")]
    impl CaptureAt {
        pub(crate) fn new(at: &'static str) -> Self {
            Self {
                at,
                done: 0,
                string: rust_alloc::string::String::default(),
            }
        }

        pub(crate) fn as_str(&self) -> Option<&str> {
            if self.done > 0 {
                Some(&self.string[self.done..])
            } else {
                None
            }
        }
    }

    #[cfg(feature = "fmt")]
    impl Write for CaptureAt {
        #[inline]
        fn write_str(&mut self, mut s: &str) -> fmt::Result {
            if self.done > 0 {
                return Ok(());
            }

            while let Some(n) = s.find('\n') {
                self.string.push_str(&s[..n]);

                if let Some(n) = self.string.find("at ") {
                    let at = &self.string[n + 3..];

                    if at.contains(self.at) {
                        self.done = n + 3;
                        return Ok(());
                    }
                }

                self.string.clear();
                s = &s[n + 1..];
            }

            self.string.push_str(s);
            Ok(())
        }
    }

    macro_rules! _rune_diagnose {
        ($($tt:tt)*) => {
            if $crate::shared::rune_assert().is_panic() {
                panic!($($tt)*);
            } else {
                tracing::trace!($($tt)*);
            }
        };
    }

    macro_rules! _rune_trace {
        ($at:expr, $tok:expr) => {{
            if $crate::shared::rune_assert().is_trace() {
                use std::backtrace::Backtrace;
                use std::fmt::Write as _;

                let bt = Backtrace::force_capture();
                let mut at = $crate::shared::CaptureAt::new($at);

                write!(at, "{bt}").with_span($tok.span)?;

                if let Some(_line) = at.as_str() {
                    tracing::trace!("{_line}: {:?}", $tok);
                }
            }
        }};
    }

    /// A macro for logging or panicking based on the current assertions model.
    ///
    /// The assertion model can be changed from logging to panicking by setting
    /// the `RUNE_ASSERT=panic` environment.
    #[doc(inline)]
    pub(crate) use _rune_diagnose as rune_diagnose;

    /// A macro for tracing a specific call site.
    ///
    /// Tracing is enabled if `RUNE_ASSERT=trace` is set.
    #[doc(inline)]
    #[cfg(feature = "fmt")]
    pub(crate) use _rune_trace as rune_trace;
}

#[cfg(not(all(debug_assertions, feature = "std")))]
mod r#impl {
    use core::fmt;

    macro_rules! _rune_diagnose {
        ($($tt:tt)*) => {
            tracing::trace!($($tt)*);
        };
    }

    macro_rules! _rune_trace {
        ($at:expr, $tok:expr) => {{
            _ = $at;
            _ = $tok;
        }};
    }

    pub(crate) struct Backtrace;

    impl Backtrace {
        #[inline(always)]
        pub(crate) fn capture() -> Self {
            Self
        }
    }

    impl fmt::Display for Backtrace {
        #[inline(always)]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "backtrace not available, missing cfg(all(debug_assertions, feature = \"std\"))"
            )
        }
    }

    /// A macro for logging or panicking based on the current assertions model.
    ///
    /// The assertion model can be changed from logging to panicking by setting
    /// the `RUNE_ASSERT=panic` environment.
    #[doc(inline)]
    pub(crate) use _rune_diagnose as rune_diagnose;

    /// A macro for tracing a specific call site.
    ///
    /// Tracing is enabled if `RUNE_ASSERT=trace` is set.
    #[doc(inline)]
    #[cfg(feature = "fmt")]
    pub(crate) use _rune_trace as rune_trace;
}

pub(crate) use self::r#impl::*;
