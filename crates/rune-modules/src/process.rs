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

use rune::alloc::clone::TryClone;
use rune::alloc::fmt::TryWrite;
use rune::alloc::Vec;
use rune::runtime::{Bytes, Formatter, Mut, Value, VmResult};
use rune::{vm_try, vm_write, Any, ContextError, Module};

use std::io;
use tokio::process;

/// A module for working with processes.
///
/// This allows spawning child processes, capturing their output, and creating pipelines.
#[rune::module(::process)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Command>()?;
    module.ty::<Child>()?;
    module.ty::<ExitStatus>()?;
    module.ty::<Output>()?;
    module.ty::<Stdio>()?;
    module.ty::<ChildStdin>()?;
    module.ty::<ChildStdout>()?;
    module.ty::<ChildStderr>()?;

    module.function_meta(Command::string_debug)?;
    module.function_meta(Command::new)?;
    module.function_meta(Command::spawn)?;
    module.function_meta(Command::arg)?;
    module.function_meta(Command::args)?;
    #[cfg(unix)]
    module.function_meta(Command::arg0)?;
    module.function_meta(Command::stdin)?;
    module.function_meta(Command::stdout)?;
    module.function_meta(Command::stderr)?;

    module.function_meta(Child::string_debug)?;
    module.function_meta(Child::stdin)?;
    module.function_meta(Child::stdout)?;
    module.function_meta(Child::stderr)?;
    module.function_meta(Child::id)?;
    module.function_meta(Child::start_kill)?;
    module.function_meta(Child::kill)?;
    module.function_meta(Child::wait)?;
    module.function_meta(Child::wait_with_output)?;

    module.function_meta(ExitStatus::string_debug)?;
    module.function_meta(ExitStatus::string_display)?;
    module.function_meta(ExitStatus::code)?;
    module.function_meta(ExitStatus::success)?;

    module.function_meta(Output::string_debug)?;
    module.function_meta(Stdio::null)?;
    module.function_meta(Stdio::inherit)?;
    module.function_meta(Stdio::piped)?;

    module.function_meta(ChildStdin::string_debug)?;
    module.function_meta(ChildStdin::try_into_stdio)?;

    module.function_meta(ChildStdout::string_debug)?;
    module.function_meta(ChildStdout::try_into_stdio)?;

    module.function_meta(ChildStderr::string_debug)?;
    module.function_meta(ChildStderr::try_into_stdio)?;

    Ok(module)
}

/// A builder for a child command to execute
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Command {
    inner: process::Command,
}

impl Command {
    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self);
    }

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

    #[cfg(unix)]
    #[rune::function(instance)]
    /// Set the first process argument, argv[0], to something other than the default executable path. (Unix only)
    fn arg0(&mut self, arg: &str) {
        self.inner.arg0(arg);
    }

    /// Sets configuration for the child process’s standard input (stdin) handle.
    #[rune::function(instance)]
    fn stdin(&mut self, stdio: Stdio) {
        self.inner.stdin(stdio.inner);
    }

    /// Sets configuration for the child process’s standard output (stdout) handle.
    #[rune::function(instance)]
    fn stdout(&mut self, stdio: Stdio) {
        self.inner.stdout(stdio.inner);
    }

    /// Sets configuration for the child process’s standard error (stderr) handle.
    #[rune::function(instance)]
    fn stderr(&mut self, stdio: Stdio) {
        self.inner.stderr(stdio.inner);
    }

    /// Spawn the command.
    #[rune::function(instance)]
    fn spawn(mut self) -> io::Result<Child> {
        Ok(Child {
            inner: Some(self.inner.spawn()?),
        })
    }
}

/// A running child process
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Child {
    // we use an option to avoid a panic if we try to complete the child process
    // multiple times.
    //
    // TODO: enapculate this pattern in some better way.
    inner: Option<process::Child>,
}

impl Child {
    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self);
    }

    /// Attempt to take the stdin of the child process.
    ///
    /// Once taken this can not be taken again.
    #[rune::function(instance)]
    fn stdin(&mut self) -> Option<ChildStdin> {
        let inner = match &mut self.inner {
            Some(inner) => inner,
            None => return None,
        };
        let stdin = inner.stdin.take()?;
        Some(ChildStdin { inner: stdin })
    }

    /// Attempt to take the stdout of the child process.
    ///
    /// Once taken this can not be taken again.
    #[rune::function(instance)]
    fn stdout(&mut self) -> Option<ChildStdout> {
        let inner = match &mut self.inner {
            Some(inner) => inner,
            None => return None,
        };
        let stdout = inner.stdout.take()?;
        Some(ChildStdout { inner: stdout })
    }

    /// Attempt to take the stderr of the child process.
    ///
    /// Once taken this can not be taken again.
    #[rune::function(instance)]
    fn stderr(&mut self) -> Option<ChildStderr> {
        let inner = match &mut self.inner {
            Some(inner) => inner,
            None => return None,
        };
        let stderr = inner.stderr.take()?;
        Some(ChildStderr { inner: stderr })
    }

    /// Attempt to get the OS process id of the child process.
    ///
    /// This will return None after the child process has completed.
    #[rune::function(instance)]
    fn id(&self) -> Option<u32> {
        match &self.inner {
            Some(inner) => inner.id(),
            None => None,
        }
    }

    #[rune::function(vm_result, instance)]
    fn start_kill(&mut self) -> io::Result<()> {
        let inner = match &mut self.inner {
            Some(inner) => inner,
            None => {
                rune::vm_panic!("already completed");
            }
        };

        inner.start_kill()
    }

    /// Sends a signal to the child process.
    #[rune::function(vm_result, instance, path = Self::kill)]
    async fn kill(mut this: Mut<Self>) -> io::Result<()> {
        let inner = match &mut this.inner {
            Some(inner) => inner,
            None => {
                rune::vm_panic!("already completed");
            }
        };

        inner.kill().await
    }

    /// Attempt to wait for the child process to exit.
    ///
    /// This will not capture output, use [`wait_with_output`] for that.
    #[rune::function(vm_result, instance)]
    async fn wait(self) -> io::Result<ExitStatus> {
        let mut inner = match self.inner {
            Some(inner) => inner,
            None => {
                rune::vm_panic!("already completed");
            }
        };

        let status = inner.wait().await?;

        Ok(ExitStatus { status })
    }

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
            status: ExitStatus {
                status: output.status,
            },
            stdout: Bytes::from_vec(Vec::try_from(output.stdout).vm?),
            stderr: Bytes::from_vec(Vec::try_from(output.stderr).vm?),
        })
    }
}

/// The output and exit status, returned by [`Child::wait_with_output`].
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Output {
    #[rune(get)]
    status: ExitStatus,
    #[rune(get)]
    stdout: Bytes,
    #[rune(get)]
    stderr: Bytes,
}

impl Output {
    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self);
    }
}

/// The exit status from a completed child process
#[derive(Debug, TryClone, Clone, Copy, Any)]
#[rune(item = ::process)]
struct ExitStatus {
    status: std::process::ExitStatus,
}

impl ExitStatus {
    #[rune::function(vm_result, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) {
        vm_write!(f, "{}", self.status);
    }

    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self);
    }

    #[rune::function]
    fn success(&self) -> bool {
        self.status.success()
    }

    #[rune::function]
    fn code(&self) -> Option<i32> {
        self.status.code()
    }
}

/// Describes what to do with a standard I/O stream for a child process when passed to the stdin, stdout, and stderr methods of Command.
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Stdio {
    inner: std::process::Stdio,
}

impl Stdio {
    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self);
    }

    /// This stream will be ignored. This is the equivalent of attaching the stream to /dev/null.
    #[rune::function(path = Self::null)]
    fn null() -> Self {
        Self {
            inner: std::process::Stdio::null(),
        }
    }

    /// The child inherits from the corresponding parent descriptor. This is the default.
    #[rune::function(path = Self::inherit)]
    fn inherit() -> Self {
        Self {
            inner: std::process::Stdio::inherit(),
        }
    }

    /// A new pipe should be arranged to connect the parent and child processes.
    #[rune::function(path = Self::piped)]
    fn piped() -> Self {
        Self {
            inner: std::process::Stdio::piped(),
        }
    }
}

macro_rules! stdio_stream {
    ($name:ident, $stream:tt) => {
        #[derive(Debug, Any)]
        #[rune(item = ::process)]
        #[doc = concat!("The ", $stream, " stream for spawned children.")]
        struct $name {
            inner: process::$name,
        }

        impl $name {
            #[rune::function(vm_result, protocol = STRING_DEBUG)]
            fn string_debug(&self, f: &mut Formatter) {
                vm_write!(f, "{:?}", self);
            }

            /// Try to convert into a `Stdio`, which allows creating a pipeline between processes.
            ///
            /// This consumes the stream, as it can only be used once.
            ///
            /// Returns a Result<Stdio>
            #[rune::function(instance)]
            fn try_into_stdio(self) -> Result<Stdio, std::io::Error> {
                Ok(Stdio {
                    inner: self.inner.try_into()?,
                })
            }
        }
    };
}
stdio_stream!(ChildStdin, "stdin");
stdio_stream!(ChildStdout, "stdout");
stdio_stream!(ChildStderr, "stderr");
