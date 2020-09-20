use runestick::{ContextError, Module, Panic, Stack, Value, VmError};
use std::cell;
use std::io;

/// Provide a bunch of `std` functions which does something appropriate to the
/// wasm context.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);
    module.function(&["print"], print_impl)?;
    module.function(&["println"], println_impl)?;
    module.raw_fn(&["dbg"], dbg_impl)?;
    Ok(module)
}

thread_local!(static OUT: cell::RefCell<io::Cursor<Vec<u8>>> = cell::RefCell::new(io::Cursor::new(Vec::new())));

/// Drain all output that has been written to `OUT`. If `OUT` contains non -
/// UTF-8, will drain but will still return `None`.
pub fn drain_output() -> Option<String> {
    OUT.with(|out| {
        let mut out = out.borrow_mut();
        let out = std::mem::take(&mut *out).into_inner();
        String::from_utf8(out).ok()
    })
}

fn print_impl(m: &str) -> Result<(), Panic> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();
        write!(out, "{}", m).map_err(Panic::custom)
    })
}

fn println_impl(m: &str) -> Result<(), Panic> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();
        writeln!(out, "{}", m).map_err(Panic::custom)
    })
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();

        for value in stack.drain_stack_top(args)? {
            writeln!(out, "{:?}", value).map_err(VmError::panic)?;
        }

        stack.push(Value::Unit);
        Ok(())
    })
}
