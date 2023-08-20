//! The `std::tuple` module.

use crate as rune;
use crate::runtime::{FromValue, OwnedTuple, Tuple, Value, VmResult};
use crate::{ContextError, Module};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["tuple"]);
    m.item_mut().docs(["The `std::tuple` module."]);
    m.ty::<OwnedTuple>()?.docs(["The tuple type."]);
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(get)?;
    Ok(m)
}

/// Returns the number of elements in the tuple.
///
/// # Examples
///
/// ```
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
/// ```
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
/// let v = [10, 40, 30];
/// assert_eq!(Some(40), v.get(1));
/// assert_eq!(Some([10, 40]), v.get(0..2));
/// assert_eq!(None, v.get(3));
/// assert_eq!(None, v.get(0..4));
/// ```
#[rune::function(instance)]
fn get(this: &Tuple, index: Value) -> VmResult<Option<Value>> {
    let slice = match index {
        Value::RangeFrom(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            this.get(start..)
        }
        Value::RangeFull(..) => this.get(..),
        Value::RangeInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..=end)
        }
        Value::RangeToInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..=end)
        }
        Value::RangeTo(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..end)
        }
        Value::Range(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..end)
        }
        value => {
            let index = vm_try!(usize::from_value(value));

            let Some(value) = this.get(index) else {
                return VmResult::Ok(None);
            };

            return VmResult::Ok(Some(value.clone()));
        }
    };

    let Some(values) = slice else {
        return VmResult::Ok(None);
    };

    VmResult::Ok(Some(Value::vec(values.to_vec())))
}
