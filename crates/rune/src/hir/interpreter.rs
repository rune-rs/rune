use crate::alloc::HashMap;
use crate::runtime::Value;

/// HIR interpreter.
#[allow(unused)]
pub(crate) struct Interpreter<'hir> {
    variables: HashMap<&'hir str, Value>,
}
