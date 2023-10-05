//! I/O module capable of capturing what's been written to a buffer.
//!
//! ```
//! use rune::{Context, ContextError};
//! use rune::modules::capture_io::{self, CaptureIo};
//!
//! let io = CaptureIo::new();
//!
//! let mut context = rune::Context::with_config(false)?;
//! context.install(capture_io::module(&io)?)?;
//! # Ok::<_, ContextError>(())
//! ```

use core::mem::take;

use ::rust_alloc::sync::Arc;

use parking_lot::Mutex;

use crate::alloc::fmt::TryWrite;
use crate::alloc::string::FromUtf8Error;
use crate::alloc::{String, Vec};
use crate::runtime::{Stack, VmError, VmResult};
use crate::{ContextError, Module, Value};

/// Provide a bunch of `std` functions that can be used during tests to capture output.
pub fn module(io: &CaptureIo) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["io"])?;

    let o = io.clone();

    module
        .function("print", move |m: &str| {
            match write!(o.inner.lock(), "{}", m) {
                Ok(()) => VmResult::Ok(()),
                Err(error) => VmResult::panic(error),
            }
        })
        .build()?;

    let o = io.clone();

    module
        .function("println", move |m: &str| {
            match writeln!(o.inner.lock(), "{}", m) {
                Ok(()) => VmResult::Ok(()),
                Err(error) => VmResult::panic(error),
            }
        })
        .build()?;

    let o = io.clone();

    module
        .raw_function("dbg", move |stack, args| {
            let mut o = o.inner.lock();
            dbg_impl(&mut o, stack, args)
        })
        .build()?;

    Ok(module)
}

/// Type which captures output from rune scripts.
#[derive(Default, Clone)]
pub struct CaptureIo {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl CaptureIo {
    /// Construct a new capture.
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain all captured I/O that has been written to output functions.
    pub fn drain(&self) -> Vec<u8> {
        let mut o = self.inner.lock();
        take(&mut *o)
    }

    cfg_std! {
        /// Drain all captured I/O that has been written to output functions into
        /// the given [Write].
        ///
        /// [Write]: std::io::Write
        pub fn drain_into<O>(&self, mut out: O) -> std::io::Result<()>
        where
            O: std::io::Write,
        {
            let mut o = self.inner.lock();
            out.write_all(o.as_slice())?;
            o.clear();
            Ok(())
        }
    }

    /// Drain all captured I/O that has been written to output functions and try
    /// to decode as UTF-8.
    pub fn drain_utf8(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.drain())
    }
}

fn dbg_impl(o: &mut Vec<u8>, stack: &mut Stack, args: usize) -> VmResult<()> {
    for value in vm_try!(stack.drain(args)) {
        vm_try!(writeln!(o, "{:?}", value).map_err(VmError::panic));
    }

    vm_try!(stack.push(Value::EmptyTuple));
    VmResult::Ok(())
}
