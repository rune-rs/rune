#[macro_export]
macro_rules! try_vec {
    () => (
        $crate::vec::Vec::new()
    );

    ($elem:expr; $n:expr) => (
        $crate::vec::try_from_elem($elem, $n)?
    );

    ($($x:expr),+ $(,)?) => (
        $crate::vec::into_vec(
            // This rustc_box is not required, but it produces a dramatic improvement in compile
            // time when constructing arrays with many elements.
            $crate::boxed::Box::try_from([$($x),+])?
        )
    );
}

#[macro_export]
macro_rules! try_format {
    ($($tt:tt)*) => {{
        (|| {
            use $crate::fmt::TryWrite;
            let mut s = $crate::alloc::string::String::new();
            core::write!(s, $($tt)*)?;
            Ok::<_, $crate::Error>(s)
        })()
    }};
}
