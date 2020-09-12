//! The `std::object` module.

use crate::{ContextError, Module, Object, Value};
use std::iter::Rev;

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "object"]);

    module.ty::<Object>()?;
    module.ty::<Iter>()?;
    module.ty::<Rev<Iter>>()?;

    module.inst_fn("len", Object::len)?;
    module.inst_fn("insert", Object::insert)?;
    module.inst_fn("clear", Object::clear)?;
    module.inst_fn("contains_key", contains_key)?;
    module.inst_fn("get", get)?;

    module.inst_fn(crate::INTO_ITER, object_iter)?;
    module.inst_fn("next", Iter::next)?;
    module.inst_fn(crate::NEXT, Iter::next)?;
    module.inst_fn(crate::INTO_ITER, Iter::into_iter)?;

    module.inst_fn("rev", Iter::rev)?;
    module.inst_fn("next", Rev::<Iter>::next)?;
    module.inst_fn("next_back", Rev::<Iter>::next_back)?;
    module.inst_fn(crate::NEXT, Rev::<Iter>::next)?;
    module.inst_fn(crate::INTO_ITER, Rev::<Iter>::into_iter)?;

    Ok(module)
}

/// An iterator over a vector.
pub struct Iter {
    iter: std::vec::IntoIter<(String, Value)>,
}

impl Iterator for Iter {
    type Item = (String, Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl DoubleEndedIterator for Iter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

fn object_iter(object: &Object) -> Iter {
    Iter {
        iter: object
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
            .into_iter(),
    }
}

fn contains_key(object: &Object, key: &str) -> bool {
    object.contains_key(key)
}

fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}

crate::__internal_impl_any!(Iter);
crate::__internal_impl_any!(Rev<Iter>, "Rev");
