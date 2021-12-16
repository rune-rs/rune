//! The native `process` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.10.3", features = ["process"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::process::module(true)?)?;
//! # Ok(())
//! # }
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

use rune::{Any, Module, ContextError};
use rune::runtime::{Bytes, Shared, Value, VmError, Protocol};
use std::fmt;
use std::io;
use tokio::process;

/// Construct the `process` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("process");
    module.ty::<Command>()?;
    module.ty::<Child>()?;
    module.ty::<ExitStatus>()?;
    module.ty::<Output>()?;

    module.function(&["Command", "new"], Command::new)?;
    module.inst_fn("spawn", Command::spawn)?;
    module.inst_fn("arg", Command::arg)?;
    module.inst_fn("args", Command::args)?;
    module.async_inst_fn("wait_with_output", Child::wait_with_output)?;
    module.inst_fn(Protocol::STRING_DISPLAY, ExitStatus::display)?;
    module.inst_fn("code", ExitStatus::code)?;
    Ok(module)
}

#[derive(Any)]
struct Command {
    inner: process::Command,
}

impl Command {
    /// Construct a new command.
    fn new(command: &str) -> Self {
        Self {
            inner: process::Command::new(command),
        }
    }

    /// Add arguments.
    fn args(&mut self, args: &[Value]) -> Result<(), VmError> {
        for arg in args {
            match arg {
                Value::String(s) => {
                    self.inner.arg(&*s.borrow_ref()?);
                }
                Value::StaticString(s) => {
                    self.inner.arg(&***s);
                }
                actual => {
                    return Err(VmError::expected::<String>(actual.type_info()?));
                }
            }
        }

        Ok(())
    }

    /// Add an argument.
    fn arg(&mut self, arg: &str) {
        self.inner.arg(arg);
    }

    /// Spawn the command.
    fn spawn(mut self) -> io::Result<Child> {
        Ok(Child {
            inner: Some(self.inner.spawn()?),
        })
    }
}

#[derive(Any)]
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
    async fn wait_with_output(self) -> Result<io::Result<Output>, VmError> {
        let inner = match self.inner {
            Some(inner) => inner,
            None => {
                return Err(VmError::panic("already completed"));
            }
        };

        let output = match inner.wait_with_output().await {
            Ok(output) => output,
            Err(error) => return Ok(Err(error)),
        };

        Ok(Ok(Output {
            status: ExitStatus { status: output.status },
            stdout: Shared::new(Bytes::from_vec(output.stdout)),
            stderr: Shared::new(Bytes::from_vec(output.stderr)),
        }))
    }
}

#[derive(Any)]
struct Output {
    #[rune(get)]
    status: ExitStatus,
    #[rune(get)]
    stdout: Shared<Bytes>,
    #[rune(get)]
    stderr: Shared<Bytes>,
}

#[derive(Clone, Copy, Any)]
struct ExitStatus {
    status: std::process::ExitStatus,
}

impl ExitStatus {
    fn display(&self, buf: &mut String) -> fmt::Result {
        use std::fmt::Write as _;
        write!(buf, "{}", self.status)
    }

    fn code(&self) -> Option<i32> {
        self.status.code()
    }
}
