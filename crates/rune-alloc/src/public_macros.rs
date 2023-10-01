#[macro_export]
macro_rules! try_vec {
    () => (
        $crate::vec::Vec::new()
    );

    ($elem:expr; $n:expr) => (
        $crate::vec::try_from_elem($elem, $n)?
    );

    ($($x:expr),+ $(,)?) => (
        $crate::slice::into_vec(
            // This rustc_box is not required, but it produces a dramatic improvement in compile
            // time when constructing arrays with many elements.
            $crate::boxed::Box::try_from([$($x),+])?
        )
    );
}

/// Creates a `String` using interpolation of runtime expressions.
///
/// The first argument `try_format!` receives is a format string. This must be a
/// string literal. The power of the formatting string is in the `{}`s
/// contained.
///
/// Additional parameters passed to `try_format!` replace the `{}`s within the
/// formatting string in the order given unless named or positional parameters
/// are used; see [`std::fmt`] for more information.
///
/// A common use for `try_format!` is concatenation and interpolation of
/// strings. The same convention is used with [`print!`] and [`write!`] macros,
/// depending on the intended destination of the string.
///
/// To convert a single value to a string, use the [`try_to_string`] method.
/// This will use the [`Display`] formatting trait.
///
/// [`std::fmt`]: ../std/fmt/index.html
/// [`print!`]: ../std/macro.print.html
/// [`write!`]: core::write
/// [`try_to_string`]: crate::string::TryToString
/// [`Display`]: core::fmt::Display
///
/// # Panics
///
/// `try_format!` panics if a formatting trait implementation returns an error. This
/// indicates an incorrect implementation since `fmt::Write for String` never
/// returns an error itself.
///
/// # Examples
///
/// ```
/// use rune::alloc::try_format;
///
/// try_format!("test");
/// try_format!("hello {}", "world!");
/// try_format!("x = {}, y = {y}", 10, y = 30);
/// let (x, y) = (1, 2);
/// try_format!("{x} + {y} = 3");
/// # Ok::<_, rune::alloc::Error>(())
/// ```
#[macro_export]
macro_rules! try_format {
    ($($tt:tt)*) => {
        $crate::fmt::try_format(format_args!($($tt)*))?
    };
}
