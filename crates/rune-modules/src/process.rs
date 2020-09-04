//! The native `process` module for the [Rune Language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.6", features = ["process"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::process::module()?)?;
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

use runestick::{Bytes, Shared, Value, VmError};
use std::fmt;
use std::io;
use tokio::process;

/// Construct the `process` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["process"]);
    module.ty(&["Command"]).build::<Command>()?;
    module.ty(&["Child"]).build::<Child>()?;
    module.ty(&["ExitStatus"]).build::<ExitStatus>()?;
    module.ty(&["Output"]).build::<Output>()?;

    module.function(&["Command", "new"], Command::new)?;
    module.inst_fn("spawn", Command::spawn)?;
    module.inst_fn("arg", Command::arg)?;
    module.inst_fn("args", Command::args)?;
    module.inst_fn(runestick::INTO_FUTURE, Child::into_future)?;
    module.async_inst_fn("wait_with_output", Child::wait_with_output)?;
    module.inst_fn(runestick::STRING_DISPLAY, ExitStatus::display)?;
    module.inst_fn("code", ExitStatus::code)?;

    module.getter("status", Output::status)?;
    module.getter("stdout", Output::stdout)?;
    module.getter("stderr", Output::stderr)?;
    Ok(module)
}

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
            inner: self.inner.spawn()?,
        })
    }
}

struct Child {
    inner: process::Child,
}

impl Child {
    /// Convert the child into a future, use for `.await`.
    fn into_future(self) -> runestick::Future {
        runestick::Future::new(async move {
            let status = match self.inner.await {
                Ok(status) => status,
                Err(error) => return Ok(Err(error)),
            };

            Ok(Ok(ExitStatus { status }))
        })
    }

    // Returns a future that will resolve to an Output, containing the exit
    // status, stdout, and stderr of the child process.
    async fn wait_with_output(self) -> io::Result<Output> {
        let inner = self.inner.wait_with_output().await?;

        Ok(Output {
            status: inner.status,
            stdout: Shared::new(Bytes::from_vec(inner.stdout)),
            stderr: Shared::new(Bytes::from_vec(inner.stderr)),
        })
    }
}

struct Output {
    status: std::process::ExitStatus,
    stdout: Shared<Bytes>,
    stderr: Shared<Bytes>,
}

impl Output {
    /// Get the exist status of the process.
    fn status(&self) -> ExitStatus {
        ExitStatus {
            status: self.status,
        }
    }

    /// Grab the stdout of the process.
    fn stdout(&mut self) -> Shared<Bytes> {
        self.stdout.clone()
    }

    /// Grab the stderr of the process.
    fn stderr(&mut self) -> Shared<Bytes> {
        self.stderr.clone()
    }
}

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

runestick::decl_external!(Command);
runestick::decl_external!(Child);
runestick::decl_external!(ExitStatus);
runestick::decl_external!(Output);
