//! The native `signal` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.3", features = ["signal"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::signal::module(true)?)?;
//! # Ok::<_, rune::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     signal::ctrl_c().await?;
//!     println("Exiting...");
//! }
//! ```

use std::io;

use rune::{Module, ContextError};

/// Construct the `signal` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("signal");
    module.function_meta(ctrl_c)?;
    Ok(module)
}

/// Completes when a "ctrl-c" notification is sent to the process.
///
/// While signals are handled very differently between Unix and Windows, both
/// platforms support receiving a signal on "ctrl-c". This function provides a
/// portable API for receiving this notification.
///
/// Once the returned future is polled, a listener is registered. The future
/// will complete on the first received `ctrl-c` **after** the initial call to
/// either `Future::poll` or `.await`.
///
/// # Tokio
/// 
/// This function is implemented using [Tokio], and requires the Tokio runtime
/// to be in scope.
/// 
/// [Tokio]: https://tokio.rs
///
/// # Caveats
///
/// On Unix platforms, the first time that a `Signal` instance is registered for
/// a particular signal kind, an OS signal-handler is installed which replaces
/// the default platform behavior when that signal is received, **for the
/// duration of the entire process**.
///
/// For example, Unix systems will terminate a process by default when it
/// receives a signal generated by "CTRL+C" on the terminal. But, when a
/// `ctrl_c` stream is created to listen for this signal, the time it arrives,
/// it will be translated to a stream event, and the process will continue to
/// execute.  **Even if this `Signal` instance is dropped, subsequent SIGINT
/// deliveries will end up captured by Tokio, and the default platform behavior
/// will NOT be reset**.
///
/// Thus, applications should take care to ensure the expected signal behavior
/// occurs as expected after listening for specific signals.
///
/// # Examples
///
/// ```rune,no_run
/// pub async fn main() {
///     println!("Waiting for ctrl-c");
///     signal::ctrl_c().await?;
///     println!("Received ctrl-c event");
/// }
/// ```
#[rune::function]
async fn ctrl_c() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}
