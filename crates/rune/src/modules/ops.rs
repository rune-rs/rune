//! The `std::ops` module.

use crate::runtime::{Protocol, Range, Value};
use crate::{ContextError, Module};

/// Construct the `std::ops` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["ops"]);
    module.ty::<Range>()?;
    module.struct_meta::<Range, 2>(["start", "end"])?;
    module.field_fn(Protocol::GET, "start", |r: &Range| r.start.clone())?;
    module.field_fn(Protocol::SET, "start", range_set_start)?;

    module.field_fn(Protocol::GET, "end", |r: &Range| r.end.clone())?;
    module.field_fn(Protocol::SET, "end", range_set_end)?;
    module.inst_fn(Protocol::INTO_ITER, Range::into_iterator)?;

    module.inst_fn("contains_int", Range::contains_int)?;
    module.inst_fn("iter", Range::into_iterator)?;

    Ok(module)
}

fn range_set_start(range: &mut Range, start: Option<Value>) {
    range.start = start;
}

fn range_set_end(range: &mut Range, end: Option<Value>) {
    range.end = end;
}
