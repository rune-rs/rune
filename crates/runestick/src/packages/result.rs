//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::ValuePtr;

fn match_err(result: &Result<ValuePtr, ValuePtr>) -> bool {
    matches!(result, Err(_))
}

fn match_ok(result: &Result<ValuePtr, ValuePtr>) -> bool {
    matches!(result, Ok(_))
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "result"]);

    module
        .ty(&["Result"])
        .build::<Result<ValuePtr, ValuePtr>>()?;

    module
        .variant(&["Result", "Err"])
        .tuple(match_err, Result::<ValuePtr, ValuePtr>::Err);

    module
        .variant(&["Result", "Ok"])
        .tuple(match_ok, Result::<ValuePtr, ValuePtr>::Ok);

    Ok(module)
}
