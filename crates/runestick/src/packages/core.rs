//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::{ContextError, Module, Value};
use std::io;
use std::io::Write as _;

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty(&["unit"]).build::<()>()?;
    module.ty(&["bool"]).build::<bool>()?;
    module.ty(&["char"]).build::<char>()?;
    module.ty(&["byte"]).build::<u8>()?;

    module.function(&["print"], |message: &str| {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        write!(stdout, "{}", message)
    })?;

    module.function(&["println"], |message: &str| {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(stdout, "{}", message)
    })?;

    module.raw_fn(&["dbg"], |stack, args| {
        for n in 0..args {
            match stack.pop() {
                Ok(value) => {
                    println!("{} = {:?}", n, value);
                }
                Err(e) => {
                    println!("{} = {}", n, e);
                }
            }
        }

        stack.push(Value::Unit);
        Ok(())
    })?;

    Ok(module)
}
