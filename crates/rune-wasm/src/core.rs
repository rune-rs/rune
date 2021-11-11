use rune::{ContextError, Module, Panic, Stack, Value, VmError};
use std::io::Write;

/// Provide a bunch of `std` functions which does something appropriate to the
/// wasm context.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["io"]);
    module.function(&["print"], print_impl)?;
    module.function(&["println"], println_impl)?;
    module.raw_fn(&["dbg"], dbg_impl)?;
    Ok(module)
}

lazy_static::lazy_static! {
    static ref OUT: parking_lot::Mutex<Vec<u8>> = parking_lot::Mutex::new(Vec::new());
}

/// Drain all output that has been written to `OUT`. If `OUT` contains non -
/// UTF-8, will drain but will still return `None`.
pub fn drain_output() -> Option<String> {
    let mut o = OUT.lock();
    let o = std::mem::take(&mut *o);
    String::from_utf8(o).ok()
}

fn print_impl(m: &str) -> Result<(), Panic> {
    write!(OUT.lock(), "{}", m).map_err(Panic::custom)
}

fn println_impl(m: &str) -> Result<(), Panic> {
    writeln!(OUT.lock(), "{}", m).map_err(Panic::custom)
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    let mut o = OUT.lock();

    for value in stack.drain_stack_top(args)? {
        writeln!(o, "{:?}", value).map_err(VmError::panic)?;
    }

    stack.push(Value::Unit);
    Ok(())
}
