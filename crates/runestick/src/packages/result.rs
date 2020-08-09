//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::Value;

fn match_err(result: &Result<Value, Value>) -> bool {
    matches!(result, Err(_))
}

fn match_ok(result: &Result<Value, Value>) -> bool {
    matches!(result, Ok(_))
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "result"]);

    module.ty(&["Result"]).build::<Result<Value, Value>>()?;

    module
        .variant(&["Result", "Err"])
        .tuple(match_err, Result::<Value, Value>::Err);

    module
        .variant(&["Result", "Ok"])
        .tuple(match_ok, Result::<Value, Value>::Ok);

    Ok(module)
}
