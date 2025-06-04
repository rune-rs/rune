/// Helper to perform the try operation over [`VmResult`].
///
/// This can be used through [`rune::function`] by enabling the `vm_result`
/// option and suffixing an expression with `<expr>.vm?`.
///
/// [`rune::function`]: macro@crate::function
/// [`VmResult`]: crate::runtime::VmResult
#[macro_export]
#[doc(hidden)]
macro_rules! __vm_try {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(err) => {
                return Err($crate::VmError::from(err));
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
/// [`rune::function`]: macro@crate::function
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
#[doc(hidden)]
macro_rules! __vm_panic {
    ($expr:expr) => {{
        return Err($crate::runtime::VmError::panic($expr));
    }};
}

/// Helper macro to perform a `write!` in a context which errors with
/// [`VmResult`] and returns `VmResult<Result<_, E>>` on write errors.
///
/// [`VmResult`]: crate::runtime::VmResult
#[macro_export]
#[doc(hidden)]
macro_rules! __vm_write {
    ($($tt:tt)*) => {
        match core::write!($($tt)*) {
            Ok(()) => Ok(()),
            Err(err) => Err($crate::runtime::VmError::from(err)),
        }
    };
}

/// Convenience macro for extracting a documentation string from documentation
/// comments.
///
/// # Examples
///
/// ```
/// let docs: [&'static str; 3] = rune::docstring! {
///     /// Hi, this is some documentation.
///     ///
///     /// I hope you like it!
/// };
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __docstring {
    ($(#[doc = $doc:expr])*) => {
        [$($doc),*]
    };
}

#[doc(inline)]
pub use __docstring as docstring;
#[doc(inline)]
pub use __vm_panic as vm_panic;
#[doc(inline)]
pub use __vm_try as vm_try;
#[doc(inline)]
pub use __vm_write as vm_write;
