//! The native `process` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["process"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::process::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use process::Command;
//!
//! fn main() {
//!     let command = Command::new("ls");
//!     command.run().await;
//! }
//! ```

use rune::{Any, Module, ContextError, vm_try};
use rune::runtime::{Bytes, Value, VmResult, Formatter};
use rune::alloc::clone::TryClone;
use rune::alloc::fmt::TryWrite;
use rune::alloc::Vec;

use std::io;
use tokio::process;

/// Construct the `process` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("process")?;
    module.ty::<Command>()?;
    module.ty::<Child>()?;
    module.ty::<ExitStatus>()?;
    module.ty::<Output>()?;

    module.function_meta(Command::new)?;
    module.function_meta(Command::spawn)?;
    module.function_meta(Command::arg)?;
    module.function_meta(Command::args)?;
    module.function_meta(Child::wait_with_output)?;
    module.function_meta(ExitStatus::string_display)?;
    module.function_meta(ExitStatus::code)?;
    Ok(module)
}

#[derive(Any)]
#[rune(item = ::process)]
struct Command {
    inner: process::Command,
}

impl Command {
    /// Construct a new command.
    #[rune::function(path = Self::new)]
    fn new(command: &str) -> Self {
        Self {
            inner: process::Command::new(command),
        }
    }

    /// Add arguments.
    #[rune::function(instance)]
    fn args(&mut self, args: &[Value]) -> VmResult<()> {
        for arg in args {
            self.inner.arg(&*vm_try!(arg.borrow_string_ref()));
        }

        VmResult::Ok(())
    }

    /// Add an argument.
    #[rune::function(instance)]
    fn arg(&mut self, arg: &str) {
        self.inner.arg(arg);
    }

    /// Spawn the command.
    #[rune::function(instance)]
    fn spawn(mut self) -> io::Result<Child> {
        Ok(Child {
            inner: Some(self.inner.spawn()?),
        })
    }
}

#[derive(Any)]
#[rune(item = ::process)]
struct Child {
    // we use an option to avoid a panic if we try to complete the child process
    // multiple times.
    //
    // TODO: enapculate this pattern in some better way.
    inner: Option<process::Child>,
}

impl Child {
    // Returns a future that will resolve to an Output, containing the exit
    // status, stdout, and stderr of the child process.
    #[rune::function(vm_result, instance)]
    async fn wait_with_output(self) -> io::Result<Output> {
        let inner = match self.inner {
            Some(inner) => inner,
            None => {
                rune::vm_panic!("already completed");
            }
        };

        let output = inner.wait_with_output().await?;

        Ok(Output {
            status: ExitStatus { status: output.status },
            stdout: Bytes::from_vec(Vec::try_from(output.stdout).vm?),
            stderr: Bytes::from_vec(Vec::try_from(output.stderr).vm?),
        })
    }
}

#[derive(Any)]
#[rune(item = ::process)]
struct Output {
    #[rune(get)]
    status: ExitStatus,
    #[rune(get)]
    stdout: Bytes,
    #[rune(get)]
    stderr: Bytes,
}

#[derive(TryClone, Clone, Copy, Any)]
#[rune(item = ::process)]
struct ExitStatus {
    status: std::process::ExitStatus,
}

impl ExitStatus {
    #[rune::function(protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.status)
    }

    #[rune::function]
    fn code(&self) -> Option<i32> {
        self.status.code()
    }
}
