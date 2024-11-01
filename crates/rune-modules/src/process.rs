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

// Documentation copied from the Tokio project under the MIT license.
// See: https://github.com/tokio-rs/tokio/blob/master/LICENSE

use rune::alloc::clone::TryClone;
use rune::alloc::fmt::TryWrite;
use rune::alloc::Vec;
use rune::runtime::{Bytes, Formatter, Mut, Value, VmResult};
use rune::{vm_try, vm_write, Any, ContextError, Module};

use std::io;
use tokio::process;

/// A module for working with processes.
///
/// This allows spawning child processes, capturing their output, and creating
/// pipelines.
///
/// # Tokio
///
/// This function is implemented using [Tokio], and requires the Tokio runtime
/// to be in scope.
///
/// [Tokio]: https://tokio.rs
#[rune::module(::process)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;

    module.ty::<Command>()?;
    module.function_meta(Command::new__meta)?;
    module.function_meta(Command::arg__meta)?;
    module.function_meta(Command::args__meta)?;
    module.function_meta(Command::debug_fmt__meta)?;
    #[cfg(unix)]
    module.function_meta(Command::arg0__meta)?;
    module.function_meta(Command::stdin__meta)?;
    module.function_meta(Command::stdout__meta)?;
    module.function_meta(Command::stderr__meta)?;
    module.function_meta(Command::kill_on_drop__meta)?;
    module.function_meta(Command::spawn__meta)?;

    module.ty::<Child>()?;
    module.function_meta(Child::debug_fmt__meta)?;
    module.function_meta(Child::stdin__meta)?;
    module.function_meta(Child::stdout__meta)?;
    module.function_meta(Child::stderr__meta)?;
    module.function_meta(Child::id__meta)?;
    module.function_meta(Child::start_kill__meta)?;
    module.function_meta(Child::kill__meta)?;
    module.function_meta(Child::wait__meta)?;
    module.function_meta(Child::wait_with_output__meta)?;

    module.ty::<ExitStatus>()?;
    module.function_meta(ExitStatus::code__meta)?;
    module.function_meta(ExitStatus::success__meta)?;
    module.function_meta(ExitStatus::display_fmt__meta)?;
    module.function_meta(ExitStatus::debug_fmt__meta)?;

    module.ty::<Output>()?;
    module.function_meta(Output::debug_fmt__meta)?;

    module.ty::<Stdio>()?;
    module.function_meta(Stdio::null__meta)?;
    module.function_meta(Stdio::inherit__meta)?;
    module.function_meta(Stdio::piped__meta)?;
    module.function_meta(Stdio::debug_fmt__meta)?;

    module.ty::<ChildStdin>()?;
    module.function_meta(ChildStdin::debug_fmt__meta)?;
    module.function_meta(ChildStdin::try_into_stdio__meta)?;

    module.ty::<ChildStdout>()?;
    module.function_meta(ChildStdout::debug_fmt__meta)?;
    module.function_meta(ChildStdout::try_into_stdio__meta)?;

    module.ty::<ChildStderr>()?;
    module.function_meta(ChildStderr::debug_fmt__meta)?;
    module.function_meta(ChildStderr::try_into_stdio__meta)?;

    Ok(module)
}

/// This structure mimics the API of [`std::process::Command`] found in the
/// standard library, but replaces functions that create a process with an
/// asynchronous variant. The main provided asynchronous functions are
/// [spawn](Command::spawn), [status](Command::status), and
/// [output](Command::output).
///
/// `Command` uses asynchronous versions of some `std` types (for example
/// [`Child`]).
///
/// [`std::process::Command`]:
///     https://doc.rust-lang.org/std/process/struct.Command.html
/// [`Child`]: struct@Child
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Command {
    inner: process::Command,
}

impl Command {
    /// Constructs a new `Command` for launching the program at path `program`,
    /// with the following default configuration:
    ///
    /// * No arguments to the program
    /// * Inherit the current process's environment
    /// * Inherit the current process's working directory
    /// * Inherit stdin/stdout/stderr for `spawn` or `status`, but create pipes
    ///   for `output`
    ///
    /// Builder methods are provided to change these defaults and otherwise
    /// configure the process.
    ///
    /// If `program` is not an absolute path, the `PATH` will be searched in an
    /// OS-defined way.
    ///
    /// The search path to be used may be controlled by setting the `PATH`
    /// environment variable on the Command, but this has some implementation
    /// limitations on Windows (see issue [rust-lang/rust#37519]).
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::Command;
    /// let command = Command::new("sh");
    /// ```
    ///
    /// [rust-lang/rust#37519]: https://github.com/rust-lang/rust/issues/37519
    #[rune::function(keep, path = Self::new)]
    fn new(command: &str) -> Self {
        Self {
            inner: process::Command::new(command),
        }
    }

    /// Adds an argument to pass to the program.
    ///
    /// Only one argument can be passed per use. So instead of:
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("sh");
    /// command.arg("-C /path/to/repo");
    /// ```
    ///
    /// usage would be:
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("sh");
    /// command.arg("-C");
    /// command.arg("/path/to/repo");
    /// ```
    ///
    /// To pass multiple arguments see [`args`].
    ///
    /// [`args`]: method@Self::args
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("ls");
    /// command.arg("-l");
    /// command.arg("-a");
    ///
    /// let output = command.output().await?;
    /// ```
    #[rune::function(keep, instance)]
    fn arg(&mut self, arg: &str) {
        self.inner.arg(arg);
    }

    /// Adds multiple arguments to pass to the program.
    ///
    /// To pass a single argument see [`arg`].
    ///
    /// [`arg`]: method@Self::arg
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("ls");
    /// command.args(["-l", "-a"]);
    ///
    /// let output = command.output().await?;
    /// ```
    #[rune::function(keep, instance)]
    fn args(&mut self, args: &[Value]) -> VmResult<()> {
        for arg in args {
            self.inner.arg(&*vm_try!(arg.borrow_string_ref()));
        }

        VmResult::Ok(())
    }

    /// Sets executable argument.
    ///
    /// Set the first process argument, `argv[0]`, to something other than the
    /// default executable path.
    #[cfg(unix)]
    #[rune::function(keep, instance)]
    fn arg0(&mut self, arg: &str) {
        self.inner.arg0(arg);
    }

    /// Sets configuration for the child process's standard input (stdin)
    /// handle.
    ///
    /// Defaults to [`inherit`].
    ///
    /// [`inherit`]: process::Stdio::inherit
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::{Command, Stdio};
    ///
    /// let command = Command::new("ls");
    /// command.stdin(Stdio::null());
    ///
    /// let output = command.output().await?;
    /// ```
    #[rune::function(keep, instance)]
    fn stdin(&mut self, stdio: Stdio) {
        self.inner.stdin(stdio.inner);
    }

    /// Sets configuration for the child process's standard output (stdout)
    /// handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and defaults
    /// to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: process::Stdio::inherit
    /// [`piped`]: process::Stdio::piped
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::{Command, Stdio};
    ///
    /// let command = Command::new("ls");
    /// command.stdout(Stdio::null());
    ///
    /// let output = command.output().await?;
    /// ```
    #[rune::function(keep, instance)]
    fn stdout(&mut self, stdio: Stdio) {
        self.inner.stdout(stdio.inner);
    }

    /// Sets configuration for the child process's standard error (stderr)
    /// handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and defaults
    /// to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: process::Stdio::inherit
    /// [`piped`]: process::Stdio::piped
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::{Command, Stdio};
    ///
    /// let command = Command::new("ls");
    /// command.stderr(Stdio::null());
    ///
    /// let output = command.output().await?;
    /// ```
    #[rune::function(keep, instance)]
    fn stderr(&mut self, stdio: Stdio) {
        self.inner.stderr(stdio.inner);
    }

    /// Controls whether a `kill` operation should be invoked on a spawned child
    /// process when its corresponding `Child` handle is dropped.
    ///
    /// By default, this value is assumed to be `false`, meaning the next
    /// spawned process will not be killed on drop, similar to the behavior of
    /// the standard library.
    ///
    /// # Caveats
    ///
    /// On Unix platforms processes must be "reaped" by their parent process
    /// after they have exited in order to release all OS resources. A child
    /// process which has exited, but has not yet been reaped by its parent is
    /// considered a "zombie" process. Such processes continue to count against
    /// limits imposed by the system, and having too many zombie processes
    /// present can prevent additional processes from being spawned.
    ///
    /// Although issuing a `kill` signal to the child process is a synchronous
    /// operation, the resulting zombie process cannot be `.await`ed inside of
    /// the destructor to avoid blocking other tasks. The tokio runtime will, on
    /// a best-effort basis, attempt to reap and clean up such processes in the
    /// background, but no additional guarantees are made with regard to how
    /// quickly or how often this procedure will take place.
    ///
    /// If stronger guarantees are required, it is recommended to avoid dropping
    /// a [`Child`] handle where possible, and instead utilize
    /// `child.wait().await` or `child.kill().await` where possible.
    #[rune::function(keep, instance)]
    pub fn kill_on_drop(&mut self, kill_on_drop: bool) {
        self.inner.kill_on_drop(kill_on_drop);
    }

    /// Executes the command as a child process, returning a handle to it.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    ///
    /// This method will spawn the child process synchronously and return a
    /// handle to a future-aware child process. The `Child` returned implements
    /// `Future` itself to acquire the `ExitStatus` of the child, and otherwise
    /// the `Child` has methods to acquire handles to the stdin, stdout, and
    /// stderr streams.
    ///
    /// All I/O this child does will be associated with the current default
    /// event loop.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// async fn run_ls() {
    ///     let command = Command::new("ls");
    ///     command.spawn()?.wait().await?;
    /// }
    /// ```
    ///
    /// # Caveats
    ///
    /// ## Dropping/Cancellation
    ///
    /// Similar to the behavior to the standard library, and unlike the futures
    /// paradigm of dropping-implies-cancellation, a spawned process will, by
    /// default, continue to execute even after the `Child` handle has been
    /// dropped.
    ///
    /// The [`Command::kill_on_drop`] method can be used to modify this behavior
    /// and kill the child process if the `Child` wrapper is dropped before it
    /// has exited.
    ///
    /// ## Unix Processes
    ///
    /// On Unix platforms processes must be "reaped" by their parent process
    /// after they have exited in order to release all OS resources. A child
    /// process which has exited, but has not yet been reaped by its parent is
    /// considered a "zombie" process. Such processes continue to count against
    /// limits imposed by the system, and having too many zombie processes
    /// present can prevent additional processes from being spawned.
    ///
    /// The tokio runtime will, on a best-effort basis, attempt to reap and
    /// clean up any process which it has spawned. No additional guarantees are
    /// made with regard to how quickly or how often this procedure will take
    /// place.
    ///
    /// It is recommended to avoid dropping a [`Child`] process handle before it
    /// has been fully `await`ed if stricter cleanup guarantees are required.
    ///
    /// [`Command`]: crate::process::Command
    /// [`Command::kill_on_drop`]: crate::process::Command::kill_on_drop
    /// [`Child`]: crate::process::Child
    ///
    /// # Errors
    ///
    /// On Unix platforms this method will fail with
    /// `std::io::ErrorKind::WouldBlock` if the system process limit is reached
    /// (which includes other applications running on the system).
    #[rune::function(keep, instance)]
    fn spawn(&mut self) -> io::Result<Child> {
        Ok(Child {
            inner: self.inner.spawn()?,
        })
    }

    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{self:?}")
    }
}

/// Representation of a child process spawned onto an event loop.
///
/// # Caveats
///
/// Similar to the behavior to the standard library, and unlike the futures
/// paradigm of dropping-implies-cancellation, a spawned process will, by
/// default, continue to execute even after the `Child` handle has been dropped.
///
/// The `Command::kill_on_drop` method can be used to modify this behavior and
/// kill the child process if the `Child` wrapper is dropped before it has
/// exited.
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Child {
    // we use an option to avoid a panic if we try to complete the child process
    // multiple times.
    inner: process::Child,
}

impl Child {
    /// The handle for writing to the child's standard input (stdin), if it has
    /// been captured. To avoid partially moving the `child` and thus blocking
    /// yourself from calling functions on `child` while using `stdin`, you
    /// might find it helpful to do:
    ///
    /// ```rune,no_run
    /// # let child = #{};
    /// let stdin = child.stdin()?;
    /// ```
    #[rune::function(keep, instance)]
    fn stdin(&mut self) -> Option<ChildStdin> {
        let inner = self.inner.stdin.take()?;
        Some(ChildStdin { inner })
    }

    /// The handle for reading from the child's standard output (stdout), if it
    /// has been captured. You might find it helpful to do
    ///
    /// ```rune,no_run
    /// # let child = #{};
    /// let stdout = child.stdout.take()?;
    /// ```
    ///
    /// to avoid partially moving the `child` and thus blocking yourself from
    /// calling functions on `child` while using `stdout`.
    #[rune::function(keep, instance)]
    fn stdout(&mut self) -> Option<ChildStdout> {
        let inner = self.inner.stdout.take()?;
        Some(ChildStdout { inner })
    }

    /// The handle for reading from the child's standard error (stderr), if it
    /// has been captured. You might find it helpful to do
    ///
    /// ```rune,no_run
    /// # let child = #{};
    /// let stderr = child.stderr()?;
    /// ```
    ///
    /// to avoid partially moving the `child` and thus blocking yourself from
    /// calling functions on `child` while using `stderr`.
    #[rune::function(keep, instance)]
    fn stderr(&mut self) -> Option<ChildStderr> {
        let inner = self.inner.stderr.take()?;
        Some(ChildStderr { inner })
    }

    /// Returns the OS-assigned process identifier associated with this child
    /// while it is still running.
    ///
    /// Once the child has been polled to completion this will return `None`.
    /// This is done to avoid confusion on platforms like Unix where the OS
    /// identifier could be reused once the process has completed.
    #[rune::function(keep, instance)]
    fn id(&self) -> Option<u32> {
        self.inner.id()
    }

    /// Attempts to force the child to exit, but does not wait for the request
    /// to take effect.
    ///
    /// On Unix platforms, this is the equivalent to sending a `SIGKILL`. Note
    /// that on Unix platforms it is possible for a zombie process to remain
    /// after a kill is sent; to avoid this, the caller should ensure that
    /// either `child.wait().await` or `child.try_wait()` is invoked
    /// successfully.
    #[rune::function(keep, instance)]
    fn start_kill(&mut self) -> io::Result<()> {
        self.inner.start_kill()
    }

    /// Forces the child to exit.
    ///
    /// This is equivalent to sending a `SIGKILL` on unix platforms.
    ///
    /// If the child has to be killed remotely, it is possible to do it using a
    /// combination of the select! macro and a `oneshot` channel. In the
    /// following example, the child will run until completion unless a message
    /// is sent on the `oneshot` channel. If that happens, the child is killed
    /// immediately using the `.kill()` method.
    ///
    /// ```rune,no_run
    /// use process::Command;
    /// # async fn wait_for_something() {}
    ///
    /// let child = Command::new("sleep");
    /// child.arg("1");
    ///
    /// let child = child.spawn();
    ///
    /// let recv = wait_for_something();
    ///
    /// select {
    ///     _ = child.wait() => {}
    ///     _ = recv => child.kill().await.expect("kill failed"),
    /// }
    /// ```
    #[rune::function(keep, instance, path = Self::kill)]
    async fn kill(mut this: Mut<Self>) -> io::Result<()> {
        this.inner.kill().await
    }

    /// Waits for the child to exit completely, returning the status that it
    /// exited with. This function will continue to have the same return value
    /// after it has been called at least once.
    ///
    /// The stdin handle to the child process, if any, will be closed
    /// before waiting. This helps avoid deadlock: it ensures that the
    /// child does not block waiting for input from the parent, while
    /// the parent waits for the child to exit.
    ///
    /// If the caller wishes to explicitly control when the child's stdin
    /// handle is closed, they may `.take()` it before calling `.wait()`:
    ///
    /// # Cancel safety
    ///
    /// This function is cancel safe.
    ///
    /// ```rune,no_run
    /// use process::{Command, Stdio};
    ///
    /// let child = Command::new("cat");
    /// child.stdin(Stdio::piped());
    ///
    /// let child = child.spawn()?;
    ///
    /// let stdin = child.stdin()?;
    ///
    /// // wait for the process to complete
    /// let _ = child.wait().await?;
    /// ```
    #[rune::function(keep, instance, path = Self::wait)]
    async fn wait(mut this: Mut<Self>) -> io::Result<ExitStatus> {
        let inner = this.inner.wait().await?;
        Ok(ExitStatus { inner })
    }

    /// Returns a future that will resolve to an `Output`, containing the exit
    /// status, stdout, and stderr of the child process.
    ///
    /// The returned future will simultaneously waits for the child to exit and
    /// collect all remaining output on the stdout/stderr handles, returning an
    /// `Output` instance.
    ///
    /// The stdin handle to the child process, if any, will be closed before
    /// waiting. This helps avoid deadlock: it ensures that the child does not
    /// block waiting for input from the parent, while the parent waits for the
    /// child to exit.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent. In
    /// order to capture the output into this `Output` it is necessary to create
    /// new pipes between parent and child. Use `stdout(Stdio::piped())` or
    /// `stderr(Stdio::piped())`, respectively, when creating a `Command`.
    #[rune::function(keep, vm_result, instance)]
    async fn wait_with_output(self) -> io::Result<Output> {
        let output = self.inner.wait_with_output().await?;

        Ok(Output {
            status: ExitStatus {
                inner: output.status,
            },
            stdout: Value::new(Bytes::from_vec(Vec::try_from(output.stdout).vm?)).vm?,
            stderr: Value::new(Bytes::from_vec(Vec::try_from(output.stderr).vm?)).vm?,
        })
    }

    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{:?}", self.inner)
    }
}

/// The output of a finished process.
///
/// This is returned in a Result by either the [`output`] method of a
/// [`Command`], or the [`wait_with_output`] method of a [`Child`] process.
///
/// [`output`]: Command::output
/// [`wait_with_output`]: Child::wait_with_output
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Output {
    /// The status (exit code) of the process.
    #[rune(get, copy)]
    status: ExitStatus,
    /// The data that the process wrote to stdout.
    #[rune(get)]
    stdout: Value,
    /// The data that the process wrote to stderr.
    #[rune(get)]
    stderr: Value,
}

impl Output {
    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{self:?}")
    }
}

/// The exit status from a completed child process
#[derive(Debug, TryClone, Clone, Copy, Any)]
#[rune(item = ::process)]
struct ExitStatus {
    inner: std::process::ExitStatus,
}

impl ExitStatus {
    /// Was termination successful? Signal termination is not considered a
    /// success, and success is defined as a zero exit status.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("mkdir");
    /// command.arg("projects");
    ///
    /// let status = command.status()?;
    ///
    /// if status.success() {
    ///     println!("'projects/' directory created");
    /// } else {
    ///     println!("failed to create 'projects/' directory: {status}");
    /// }
    /// ```
    #[rune::function(keep)]
    fn success(&self) -> bool {
        self.inner.success()
    }

    /// Returns the exit code of the process, if any.
    ///
    /// In Unix terms the return value is the **exit status**: the value passed to `exit`, if the
    /// process finished by calling `exit`.  Note that on Unix the exit status is truncated to 8
    /// bits, and that values that didn't come from a program's call to `exit` may be invented by the
    /// runtime system (often, for example, 255, 254, 127 or 126).
    ///
    /// On Unix, this will return `None` if the process was terminated by a signal.
    /// [`ExitStatusExt`](crate::os::unix::process::ExitStatusExt) is an
    /// extension trait for extracting any such signal, and other details, from the `ExitStatus`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use process::Command;
    ///
    /// let command = Command::new("mkdir");
    /// command.arg("projects");
    ///
    /// let status = command.status().await?;
    ///
    /// match status.code() {
    ///     Some(code) => println!("Exited with status code: {code}"),
    ///     None => println!("Process terminated by signal")
    /// }
    /// ```
    #[rune::function(keep)]
    fn code(&self) -> Option<i32> {
        self.inner.code()
    }

    #[rune::function(keep, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.inner)
    }

    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{:?}", self.inner)
    }
}

/// Describes what to do with a standard I/O stream for a child process when passed to the stdin, stdout, and stderr methods of Command.
#[derive(Debug, Any)]
#[rune(item = ::process)]
struct Stdio {
    inner: std::process::Stdio,
}

impl Stdio {
    /// This stream will be ignored. This is the equivalent of attaching the stream to /dev/null.
    #[rune::function(keep, path = Self::null)]
    fn null() -> Self {
        Self {
            inner: std::process::Stdio::null(),
        }
    }

    /// The child inherits from the corresponding parent descriptor. This is the default.
    #[rune::function(keep, path = Self::inherit)]
    fn inherit() -> Self {
        Self {
            inner: std::process::Stdio::inherit(),
        }
    }

    /// A new pipe should be arranged to connect the parent and child processes.
    #[rune::function(keep, path = Self::piped)]
    fn piped() -> Self {
        Self {
            inner: std::process::Stdio::piped(),
        }
    }

    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{:?}", self.inner)
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
            /// Try to convert into a `Stdio`, which allows creating a pipeline between processes.
            ///
            /// This consumes the stream, as it can only be used once.
            ///
            /// Returns a Result<Stdio>
            #[rune::function(keep, instance)]
            fn try_into_stdio(self) -> io::Result<Stdio> {
                Ok(Stdio {
                    inner: self.inner.try_into()?,
                })
            }

            #[rune::function(keep, protocol = DEBUG_FMT)]
            fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
                vm_write!(f, "{:?}", self.inner)
            }
        }
    };
}
stdio_stream!(ChildStdin, "stdin");
stdio_stream!(ChildStdout, "stdout");
stdio_stream!(ChildStderr, "stderr");
