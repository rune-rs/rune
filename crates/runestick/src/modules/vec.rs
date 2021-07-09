//! The `std::vec` module.

use crate::{ContextError, Module, Protocol, Value, Vec};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["vec"]);

    module.ty::<Vec>()?;

    module.function(&["Vec", "new"], Vec::new)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("clone", Vec::clone)?;
    module.inst_fn("extend", Vec::extend)?;
    module.inst_fn("get", vec_get)?;
    module.inst_fn("iter", Vec::into_iterator)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("pop", Vec::pop)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("remove", Vec::remove)?;
    module.inst_fn("sort_by", sort_by)?;
    module.inst_fn("insert", Vec::insert)?;
    module.inst_fn(Protocol::INTO_ITER, Vec::into_iterator)?;
    module.inst_fn(Protocol::INDEX_SET, Vec::set)?;

    // TODO: parameterize with generics.
    module.inst_fn("sort_int", sort_int)?;

    Ok(module)
}

/// Sort a vector of integers.
fn sort_int(vec: &mut Vec) {
    vec.sort_by(|a, b| match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
        // NB: fall back to sorting by address.
        _ => (a as *const _ as usize).cmp(&(b as *const _ as usize)),
    });
}

fn vec_get(vec: &Vec, index: usize) -> Option<Value> {
    vec.get(index).cloned()
}

fn sort_by(vec: &mut Vec, comparator: &crate::Function) {
    vec.sort_by(|a, b| {
        comparator
            .call::<_, std::cmp::Ordering>((a, b))
            .expect("an ordering")
    })
}
