//! I/O module ignoring everything written to output.
//!
//! ```
//! use rune::{Context, ContextError};
//! use rune::modules::disable_io;
//!
//! let mut context = rune::Context::with_config(false)?;
//! context.install(disable_io::module()?)?;
//! # Ok::<_, ContextError>(())
//! ```

use crate::runtime::{Stack, VmResult};
use crate::{ContextError, Module};

/// Provide a bunch of `std::io` functions which will cause any output to be ignored.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["io"]);

    module.function(["print"], move |_: &str| {})?;

    module.function(["println"], move |_: &str| {})?;

    module.raw_fn(["dbg"], move |stack: &mut Stack, args: usize| {
        // NB: still need to maintain the stack.
        drop(vm_try!(stack.drain(args)));
        stack.push(());
        VmResult::Ok(())
    })?;

    Ok(module)
}
