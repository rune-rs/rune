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

    module.inst_fn("iter", Object::into_iterator)?;
    module.inst_fn(crate::INTO_ITER, Object::into_iterator)?;
    Ok(module)
}

fn contains_key(object: &Object, key: &str) -> bool {
    object.contains_key(key)
}

fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}
