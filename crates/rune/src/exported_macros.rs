/// Helper to perform the try operation over [`VmResult`].
///
/// This can be used through [`rune::function`] by enabling the `vm_result`
/// option and suffixing an expression with `<expr>.vm?`.
///
/// [`rune::function`]: crate::function
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

/// Helper to cause a panic.
///
/// This simply returns a [`VmResult`], but the macro is provided to play nicely
/// with [`rune::function`], since a regular return would otherwise be
/// transformed.
///
/// [`rune::function`]: crate::function
/// [`VmResult`]: crate::runtime::VmResult
///
/// # Examples
///
/// ```
/// use rune::vm_panic;
///
/// #[rune::function(vm_result)]
/// fn hello(panic: bool) {
///     if panic {
///        vm_panic!("I was told to panic");
///     }
/// }
/// ```
#[macro_export]
macro_rules! vm_panic {
    ($expr:expr) => {{
        return $crate::runtime::VmResult::panic($expr);
    }};
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
