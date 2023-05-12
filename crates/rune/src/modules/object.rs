//! The `std::object` module.

use crate::no_std::prelude::*;

use crate::runtime::{Iterator, Object, Protocol, Value};
use crate::{ContextError, Module};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["object"]);

    module.ty::<Object>()?;

    module.function_meta(Object::__new__meta)?;
    module.function_meta(Object::__with_capacity__meta)?;
    module.function_meta(Object::__len__meta)?;
    module.function_meta(Object::__is_empty__meta)?;
    module.associated_function("insert", Object::insert)?;
    module.associated_function("remove", remove)?;
    module.associated_function("clear", Object::clear)?;
    module.associated_function("contains_key", contains_key)?;
    module.associated_function("get", get)?;

    module.associated_function("iter", Object::into_iterator)?;
    module.associated_function(Protocol::INTO_ITER, Object::into_iterator)?;
    module.associated_function("keys", keys)?;
    module.associated_function("values", values)?;
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
