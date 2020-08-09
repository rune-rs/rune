//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::Value;

fn match_some(option: &Option<Value>) -> bool {
    matches!(option, Some(_))
}

fn match_none(option: &Option<Value>) -> bool {
    matches!(option, None)
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);

    module.ty(&["Option"]).build::<Option<Value>>()?;

    module
        .variant(&["Option", "Some"])
        .tuple(match_some, Option::<Value>::Some);

    module
        .variant(&["Option", "None"])
        .tuple(match_none, || Option::<Value>::None);

    Ok(module)
}
