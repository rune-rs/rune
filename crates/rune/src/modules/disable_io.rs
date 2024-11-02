//! I/O methods which will cause any output to be ignored.
//!
//! # Examples
//!
//! ```no_run
//! use rune::Context;
//! use rune::modules::disable_io;
//!
//! let mut context = rune::Context::with_config(false)?;
//! context.install(disable_io::module()?)?;
//! # Ok::<_, rune::ContextError>(())
//! ```

use crate as rune;
use crate::runtime::{InstAddress, Memory, Output, VmResult};
use crate::{ContextError, Module};

/// I/O methods which will cause any output to be ignored.
#[rune::module(::std::io)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;

    module.function("print", move |_: &str| {}).build()?;
    module.function("println", move |_: &str| {}).build()?;

    module
        .raw_function(
            "dbg",
            move |stack: &mut dyn Memory, _: InstAddress, _: usize, out: Output| {
                vm_try!(out.store(stack, ()));
                VmResult::Ok(())
            },
        )
        .build()?;

    Ok(module)
}
