//! A trait used for types which can be called.
//!
//! This trait allows for memory [`limits`] and [`budgets`] to be combined.
//!
//! [`limits`]: crate::limit
//! [`budgets`]: ../../runtime/budget/index.html

/// A trait used for types which can be called.
///
/// This trait allows for memory [`limits`] and [`budgets`] to be combined.
///
/// [`limits`]: crate::limit
/// [`budgets`]: ../../runtime/budget/index.html
pub trait Callable {
    /// Output of the callable.
    type Output;

    /// Call and consume the callable.
    fn call(self) -> Self::Output;
}

/// Blanket implementation for closures.
impl<T, O> Callable for T
where
    T: FnOnce() -> O,
{
    type Output = O;

    fn call(self) -> Self::Output {
        self()
    }
}
