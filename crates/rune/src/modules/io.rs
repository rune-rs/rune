//! The `std::io` module.

use crate::runtime::{Panic, Protocol, Stack, Value, VmError};
use crate::{ContextError, Module};
use std::fmt;
use std::fmt::Write as _;
use std::io;
use std::io::Write as _;

/// Construct the `std::io` module.
pub fn module(stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["io"]);

    module.ty::<io::Error>()?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_io_error)?;

    if stdio {
        module.function(["print"], print_impl)?;
        module.function(["println"], println_impl)?;
        module.raw_fn(["dbg"], dbg_impl)?;
    }

    Ok(module)
}

fn format_io_error(error: &std::io::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for value in stack.drain(args)? {
        writeln!(stdout, "{:?}", value).map_err(VmError::panic)?;
    }

    stack.push(Value::Unit);
    Ok(())
}

fn print_impl(m: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", m).map_err(Panic::custom)
}

fn println_impl(m: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{}", m).map_err(Panic::custom)
}
