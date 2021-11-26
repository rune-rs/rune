//! Utilities related to testing

use parking_lot::Mutex;
use rune::runtime::VmError;
use rune::{ContextError, Module, Panic, Stack, Value};
use std::io::Write;
use std::string::FromUtf8Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct CaptureIo {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl CaptureIo {
    /// Drain all output that has been written to output functions as UTF-8.
    pub fn drain_utf8(&self) -> Result<String, FromUtf8Error> {
        let mut o = self.inner.lock();
        let o = std::mem::take(&mut *o);
        String::from_utf8(o)
    }
}

/// Provide a bunch of `std` functions that can be used during tests to capture output.
pub fn capture_io() -> Result<(Module, CaptureIo), ContextError> {
    let io = CaptureIo {
        inner: Arc::new(Mutex::new(Vec::new())),
    };

    let mut module = Module::with_crate_item("std", &["io"]);

    let o = io.clone();

    module.function(&["print"], move |m: &str| {
        write!(o.inner.lock(), "{}", m).map_err(Panic::custom)
    })?;

    let o = io.clone();

    module.function(&["println"], move |m: &str| {
        writeln!(o.inner.lock(), "{}", m).map_err(Panic::custom)
    })?;

    let o = io.clone();

    module.raw_fn(&["dbg"], move |stack, args| {
        let mut o = o.inner.lock();
        dbg_impl(&mut *o, stack, args)
    })?;

    Ok((module, io))
}

fn dbg_impl<O>(o: &mut O, stack: &mut Stack, args: usize) -> Result<(), VmError>
where
    O: Write,
{
    for value in stack.drain_stack_top(args)? {
        writeln!(o, "{:?}", value).map_err(VmError::panic)?;
    }

    stack.push(Value::Unit);
    Ok(())
}
