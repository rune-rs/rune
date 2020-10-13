//! The `std::object` module.

use crate::{ContextError, Module, Object, Value};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "object"]);

    module.ty::<Object>()?;

    module.inst_fn("len", Object::len)?;
    module.inst_fn("insert", Object::insert)?;
    module.inst_fn("clear", Object::clear)?;
    module.inst_fn("contains_key", contains_key)?;
    module.inst_fn("get", get)?;

    module.inst_fn("iter", object_iter)?;
    module.inst_fn(crate::INTO_ITER, object_iter)?;
    Ok(module)
}

fn object_iter(object: &Object) -> crate::Iterator {
    crate::Iterator::from("std::object::Iter", object.clone().into_iter())
}

fn contains_key(object: &Object, key: &str) -> bool {
    object.contains_key(key)
}

fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}
