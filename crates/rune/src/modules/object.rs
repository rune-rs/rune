//! The `std::object` module.

use crate::no_std::prelude::*;

use crate::runtime::{Iterator, Object, Protocol, Value};
use crate::{ContextError, Module};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["object"]);

    module.ty::<Object>()?;

    module.inst_fn("len", Object::len)?;
    module.inst_fn("insert", Object::insert)?;
    module.inst_fn("remove", remove)?;
    module.inst_fn("clear", Object::clear)?;
    module.inst_fn("contains_key", contains_key)?;
    module.inst_fn("get", get)?;

    module.inst_fn("iter", Object::into_iterator)?;
    module.inst_fn(Protocol::INTO_ITER, Object::into_iterator)?;
    module.inst_fn("keys", keys)?;
    module.inst_fn("values", values)?;
    Ok(module)
}

fn contains_key(object: &Object, key: &str) -> bool {
    object.contains_key(key)
}

fn remove(object: &mut Object, key: &str) -> Option<Value> {
    object.remove(key)
}

fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}

fn keys(object: &Object) -> Iterator {
    let iter = object.keys().cloned().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::object::Keys", iter)
}

fn values(object: &Object) -> Iterator {
    let iter = object.values().cloned().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::object::Values", iter)
}
