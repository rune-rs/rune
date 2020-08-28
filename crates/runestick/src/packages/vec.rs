//! Package containing array functions.

use crate::{ContextError, Module, Value};

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

fn vec_iter(vec: &[Value]) -> Iter {
    Iter {
        iter: vec.to_vec().into_iter(),
    }
}

// NB: decl_internal! prevents iterator from leaving the VM since the owned type
// doesn't implement `FromValue`.
decl_external!(Iter);

/// Get the module for the array package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty(&["Vec"]).build::<Vec<Value>>()?;
    module.ty(&["Iter"]).build::<Iter>()?;

    module.function(&["Vec", "new"], Vec::<Value>::new)?;
    module.inst_fn("iter", vec_iter)?;
    module.inst_fn("len", Vec::<Value>::len)?;
    module.inst_fn("push", Vec::<Value>::push)?;
    module.inst_fn("clear", Vec::<Value>::clear)?;
    module.inst_fn("pop", Vec::<Value>::pop)?;
    module.inst_fn(crate::NEXT, Iter::next)?;
    Ok(module)
}
