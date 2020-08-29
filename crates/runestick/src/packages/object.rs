//! Package containing object functions.

use crate::{ContextError, Module, Object, Value};

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

fn object_iter(object: &Object<Value>) -> Iter {
    Iter {
        iter: object
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
            .into_iter(),
    }
}

decl_external!(Iter);

/// Get the module for the object package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "object"]);

    module.ty(&["Object"]).build::<Object<Value>>()?;
    module.ty(&["Iter"]).build::<Iter>()?;

    module.inst_fn("len", Object::<Value>::len)?;
    module.inst_fn("insert", Object::<Value>::insert)?;
    module.inst_fn("clear", Object::<Value>::clear)?;
    module.inst_fn(crate::INTO_ITER, object_iter)?;
    module.inst_fn(crate::NEXT, Iter::next)?;
    module.inst_fn(crate::INTO_ITER, Iter::into_iter)?;
    Ok(module)
}
