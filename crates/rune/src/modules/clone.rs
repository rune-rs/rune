//! The cloning trait for Rune.

use crate as rune;
use crate::runtime::{Protocol, Value, VmResult};
use crate::{ContextError, Module};

/// Cloning for Rune.
///
/// This module defines methods and types used when cloning values.
///
/// By default all values in rune are structurally shared, so in order to get a
/// unique instance of it you must clone it.
#[rune::module(::std::clone)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;
    m.function_meta(clone)?;

    let mut t = m.define_trait(["Clone"])?;

    t.docs(docstring! {
        /// The `Clone` trait is used to explicitly clone values.
        ///
        /// # Examples
        ///
        /// ```rune
        /// let a = 42;
        /// let b = a.clone();
        ///
        /// assert_eq!(a, b);
        /// ```
    })?;

    t.handler(|cx| {
        _ = cx.find(&Protocol::CLONE)?;
        Ok(())
    })?;

    t.function("clone")?
        .argument_types::<(Value,)>()?
        .return_type::<Value>()?
        .docs(docstring! {
            /// Clone the specified `value`.
            ///
            /// # Examples
            ///
            /// ```rune
            /// let a = 42;
            /// let b = a;
            /// let c = a.clone();
            ///
            /// a += 1;
            ///
            /// assert_eq!(a, 43);
            /// assert_eq!(b, 42);
            /// assert_eq!(c, 42);
            /// ```
        })?;

    Ok(m)
}

/// Clone the specified `value`.
///
/// # Examples
///
/// ```rune
/// let a = 42;
/// let b = a;
/// let c = clone(a);
///
/// a += 1;
///
/// assert_eq!(a, 43);
/// assert_eq!(b, 42);
/// assert_eq!(c, 42);
/// ```
#[rune::function]
fn clone(value: Value) -> VmResult<Value> {
    value.clone_()
}
