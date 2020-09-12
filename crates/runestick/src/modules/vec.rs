//! The `std::vec` module.

use crate::{ContextError, Module, Value};
use std::iter::Rev;

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty::<Vec<Value>>()?;
    module.ty::<Iter>()?;
    module.ty::<Rev<Iter>>()?;

    module.function(&["Vec", "new"], Vec::<Value>::new)?;
    module.inst_fn("iter", vec_iter)?;
    module.inst_fn("len", Vec::<Value>::len)?;
    module.inst_fn("push", Vec::<Value>::push)?;
    module.inst_fn("clear", Vec::<Value>::clear)?;
    module.inst_fn("pop", Vec::<Value>::pop)?;

    module.inst_fn(crate::INTO_ITER, vec_iter)?;
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
    iter: std::vec::IntoIter<Value>,
}

impl Iterator for Iter {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        self.iter.next()
    }
}

impl DoubleEndedIterator for Iter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

fn vec_iter(vec: &[Value]) -> Iter {
    Iter {
        iter: vec.to_vec().into_iter(),
    }
}

crate::__internal_impl_any!(Iter, "Iter");
crate::__internal_impl_any!(Rev<Iter>, "Rev");
