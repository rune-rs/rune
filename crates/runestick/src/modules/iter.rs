//! The `std::iter` module.

use crate::{
    ContextError, FromValue as _, Iterator, Module, Object, Protocol, Value, Vec, VmError,
};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "iter"]);
    module.ty::<Iterator>()?;

    module.inst_fn("chain", Iterator::chain)?;
    module.inst_fn("collect_object", collect_object)?;
    module.inst_fn("collect_vec", collect_vec)?;
    module.inst_fn("enumerate", Iterator::enumerate)?;
    module.inst_fn("filter", Iterator::filter)?;
    module.inst_fn("map", Iterator::map)?;
    module.inst_fn("flat_map", Iterator::flat_map)?;
    module.inst_fn("next_back", Iterator::next_back)?;
    module.inst_fn("next", Iterator::next)?;
    module.inst_fn("rev", Iterator::rev)?;
    module.inst_fn("take", Iterator::take)?;
    module.inst_fn("peekable", Iterator::peekable)?;
    module.inst_fn("peek", Iterator::peek)?;
    module.inst_fn("size_hint", Iterator::size_hint)?;
    module.inst_fn(Protocol::NEXT, Iterator::next)?;
    module.inst_fn(Protocol::INTO_ITER, <Iterator as From<Iterator>>::from)?;

    module.function(&["range"], new_range)?;
    Ok(module)
}

fn new_range(start: i64, end: i64) -> Iterator {
    Iterator::from_double_ended("std::iter::Range", start..end)
}

fn collect_vec(it: Iterator) -> Result<Vec, VmError> {
    Ok(Vec::from(it.collect::<Value>()?))
}

fn collect_object(mut it: Iterator) -> Result<Object, VmError> {
    let (cap, _) = it.size_hint();
    let mut object = Object::with_capacity(cap);

    while let Some(value) = it.next()? {
        let (key, value) = <(String, Value)>::from_value(value)?;
        object.insert(key, value);
    }

    Ok(object)
}
