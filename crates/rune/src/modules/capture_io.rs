//! I/O module capable of capturing what's been written to a buffer.
//!
//! # Examples
//!
//! ```no_run
//! use rune::Context;
//! use rune::modules::capture_io::{self, CaptureIo};
//!
//! let io = CaptureIo::new();
//!
//! let mut context = rune::Context::with_config(false)?;
//! context.install(capture_io::module(&io)?)?;
//! # Ok::<_, rune::ContextError>(())
//! ```

use core::mem::take;

use rust_alloc::sync::Arc;

use parking_lot::Mutex;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::string::FromUtf8Error;
use crate::alloc::{String, Vec};
use crate::runtime::{InstAddress, Memory, Output, VmError};
use crate::{ContextError, Module, Value};

/// I/O module capable of capturing what's been written to a buffer.
#[rune::module(::std::io)]
pub fn module(io: &CaptureIo) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module__meta)?;

    let o = io.clone();

    module
        .function("print", move |m: &str| {
            write!(o.inner.lock(), "{}", m).map_err(VmError::panic)
        })
        .build()?;

    let o = io.clone();

    module
        .function("println", move |m: &str| {
            writeln!(o.inner.lock(), "{}", m).map_err(VmError::panic)
        })
        .build()?;

    let o = io.clone();

    module
        .raw_function("dbg", move |stack, addr, args, output| {
            let mut o = o.inner.lock();
            dbg_impl(&mut o, stack, addr, args, output)
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

    /// Test if capture is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
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

fn dbg_impl(
    o: &mut Vec<u8>,
    stack: &mut dyn Memory,
    addr: InstAddress,
    args: usize,
    out: Output,
) -> Result<(), VmError> {
    for value in stack.slice_at(addr, args)? {
        writeln!(o, "{value:?}").map_err(VmError::panic)?;
    }

    out.store(stack, Value::unit)?;
    Ok(())
}
