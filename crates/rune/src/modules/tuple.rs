//! The `std::tuple` module.

use crate as rune;
use crate::runtime::{Tuple, Value, Vec, VmResult};
use crate::{ContextError, Module};

/// Dynamic tuples.
#[rune::module(::std::tuple)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta);
    m.ty::<Tuple>()?.docs(["The tuple type."]);
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(get)?;
    Ok(m)
}

/// Returns the number of elements in the tuple.
///
/// # Examples
///
/// ```rune
/// let a = (1, 2, 3);
/// assert_eq!(a.len(), 3);
/// ```
#[rune::function(instance)]
fn len(this: &Tuple) -> usize {
    this.len()
}

/// Returns `true` if the tuple has a length of 0.
///
/// # Examples
///
/// ```rune
/// let a = (1, 2, 3);
/// assert!(!a.is_empty());
///
/// let a = ();
/// assert!(a.is_empty());
/// ```
#[rune::function(instance)]
fn is_empty(this: &Tuple) -> bool {
    this.is_empty()
}

/// Returns a reference to an element or subslice depending on the type of
/// index.
///
/// - If given a position, returns a reference to the element at that position
///   or `None` if out of bounds.
/// - If given a range, returns the subslice corresponding to that range, or
///   `None` if out of bounds.
///
/// # Examples
///
/// ```rune
/// let v = (10, 40, 30);
/// assert_eq!(Some(40), v.get(1));
/// assert_eq!(Some([10, 40]), v.get(0..2));
/// assert_eq!(None, v.get(3));
/// assert_eq!(None, v.get(0..4));
/// ```
#[rune::function(instance)]
fn get(this: &Tuple, index: Value) -> VmResult<Option<Value>> {
    Vec::index_get(this, index)
}
