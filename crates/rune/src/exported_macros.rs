/// Helper to perform the try operation over [`VmResult`].
///
/// [`VmResult`]: crate::runtime::VmResult
#[macro_export]
macro_rules! vm_try {
    ($expr:expr) => {
        match $crate::runtime::try_result($expr) {
            $crate::runtime::VmResult::Ok(value) => value,
            $crate::runtime::VmResult::Err(err) => {
                return $crate::runtime::VmResult::Err($crate::runtime::VmError::from(err));
            }
        }
    };
}

/// Helper macro to perform a `write!` in a context which errors with
/// [`VmResult`] and returns `VmResult<Result<_, E>>` on write errors.
///
/// [`VmResult`]: crate::runtime::VmResult
#[macro_export]
macro_rules! vm_write {
    ($($tt:tt)*) => {
        match core::write!($($tt)*) {
            Ok(()) => (),
            Err(err) => {
                return $crate::runtime::VmResult::Err($crate::runtime::VmError::from(err));
            }
        }
    };
}
