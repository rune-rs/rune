//! The `std::iter` module.

use crate::runtime::{FromValue, Iterator, Object, Protocol, Tuple, TypeOf, Value, Vec, VmError};
use crate::{ContextError, Module, Params};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["iter"]);
    module.ty::<Iterator>()?;

    // Sorted for ease of finding
    module.inst_fn("chain", Iterator::chain)?;
    module.inst_fn(Params("collect", [Object::type_hash()]), collect_object)?;
    module.inst_fn(Params("collect", [Vec::type_hash()]), collect_vec)?;
    module.inst_fn(Params("collect", [Tuple::type_hash()]), collect_tuple)?;
    module.inst_fn("enumerate", Iterator::enumerate)?;
    module.inst_fn("filter", Iterator::filter)?;
    module.inst_fn("find", Iterator::find)?;
    module.inst_fn("flat_map", Iterator::flat_map)?;
    module.inst_fn("map", Iterator::map)?;
    module.inst_fn("next", Iterator::next)?;
    module.inst_fn("next_back", Iterator::next_back)?;
    module.inst_fn("peek", Iterator::peek)?;
    module.inst_fn("peekable", Iterator::peekable)?;
    module.inst_fn("product", Iterator::product)?;
    module.inst_fn("fold", Iterator::fold)?;
    module.inst_fn("rev", Iterator::rev)?;
    module.inst_fn("size_hint", Iterator::size_hint)?;
    module.inst_fn("sum", Iterator::sum)?;
    module.inst_fn("skip", Iterator::skip)?;
    module.inst_fn("take", Iterator::take)?;
    module.inst_fn("count", Iterator::count)?;
    module.inst_fn("all", Iterator::all)?;
    module.inst_fn(Protocol::NEXT, Iterator::next)?;
    module.inst_fn(Protocol::INTO_ITER, <Iterator as From<Iterator>>::from)?;

    module.function(&["range"], new_range)?;
    module.function(&["empty"], new_empty)?;
    module.function(&["once"], new_once)?;
    Ok(module)
}

fn new_empty() -> Iterator {
    Iterator::empty()
}

fn new_once(v: Value) -> Iterator {
    Iterator::once(v)
}

fn new_range(start: i64, end: i64) -> Iterator {
    Iterator::from_double_ended("std::iter::Range", start..end)
}

fn collect_vec(it: Iterator) -> Result<Vec, VmError> {
    Ok(Vec::from(it.collect::<Value>()?))
}

fn collect_tuple(it: Iterator) -> Result<Tuple, VmError> {
    Ok(Tuple::from(it.collect::<Value>()?))
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
