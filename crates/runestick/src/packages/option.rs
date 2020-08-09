//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::ValuePtr;

fn match_some(option: &Option<ValuePtr>) -> bool {
    matches!(option, Some(_))
}

fn match_none(option: &Option<ValuePtr>) -> bool {
    matches!(option, None)
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);

    module.ty(&["Option"]).build::<Option<ValuePtr>>()?;

    module
        .variant(&["Option", "Some"])
        .tuple(match_some, Option::<ValuePtr>::Some);

    module
        .variant(&["Option", "None"])
        .tuple(match_none, || Option::<ValuePtr>::None);

    Ok(module)
}
