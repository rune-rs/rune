mod capture_io;

use rune::compile::{IntoComponent, ItemBuf};
use rune::runtime::{Args, VmError, VmResult};
use rune::{termcolor, BuildError, Context, Diagnostics, FromValue, Source, Sources, Unit, Vm};
use std::sync::Arc;
use thiserror::Error;

/// An error that can be raised during testing.
#[derive(Debug, Error)]
pub enum RunError {
    /// A load error was raised during testing.
    #[error("build error")]
    BuildError(BuildError),
    /// A virtual machine error was raised during testing.
    #[error("vm error: {0}")]
    VmError(VmError),
}

impl RunError {
    /// Unpack into a vm error or panic with the given message.
    pub fn expect_vm_error(self, msg: &str) -> VmError {
        match self {
            Self::VmError(error) => error,
            _ => panic!("{}", msg),
        }
    }
}

/// Compile the given source into a unit and collection of warnings.
#[doc(hidden)]
pub fn compile_helper(source: &str, diagnostics: &mut Diagnostics) -> Result<Unit, BuildError> {
    let context = rune::Context::with_default_modules().expect("setting up default modules");

    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(diagnostics)
        .build()?;

    Ok(unit)
}

/// Construct a virtual machine for the given sources.
#[doc(hidden)]
pub fn vm(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
) -> Result<Vm, RunError> {
    let unit = rune::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .build()
        .map_err(RunError::BuildError)?;

    let context = Arc::new(context.runtime());
    Ok(Vm::new(context, Arc::new(unit)))
}

/// Call the specified function in the given script sources.
#[doc(hidden)]
pub fn run_helper<N, A, T>(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    ::futures_executor::block_on(async move {
        let mut vm = vm(context, sources, diagnostics)?;

        let mut execute = vm
            .execute(&ItemBuf::with_item(function), args)
            .map_err(RunError::VmError)?;
        let output = execute
            .async_complete()
            .await
            .into_result()
            .map_err(RunError::VmError)?;

        match T::from_value(output) {
            VmResult::Ok(output) => Ok(output),
            VmResult::Err(err) => Err(RunError::VmError(err)),
        }
    })
}

#[doc(hidden)]
pub fn sources(source: &str) -> Sources {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));
    sources
}

/// Run the given source with diagnostics being printed to stderr.
pub fn run<N, A, T>(context: &Context, source: &str, function: N, args: A) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));

    let mut diagnostics = Default::default();

    let e = match run_helper(context, &mut sources, &mut diagnostics, function, args) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };

    let mut writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Never);

    match &e {
        RunError::BuildError(..) => {
            diagnostics
                .emit(&mut writer, &sources)
                .expect("emit diagnostics");
        }
        RunError::VmError(e) => {
            e.emit(&mut writer, &sources).expect("emit diagnostics");
        }
    }

    Err(e)
}

/// Helper function to construct a context and unit from a Rune source for
/// testing purposes.
///
/// This is primarily used in examples.
pub fn build(context: &Context, source: &str) -> rune::Result<Arc<Unit>> {
    let mut sources = Sources::new();
    sources.insert(Source::new("source", source));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    Ok(Arc::new(result?))
}

/// Construct a rune virtual machine from the given program.
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
/// use rune::Value;
///
/// let mut vm = rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// ```
macro_rules! rune_vm {
    ($($tt:tt)*) => {{
        let context = rune::Context::with_default_modules().expect("failed to build context");
        let mut diagnostics = Default::default();
        let mut sources = $crate::sources(stringify!($($tt)*));
        $crate::vm(&context, &mut sources, &mut diagnostics).expect("program to compile successfully")
    }};
}

/// Construct a rune virtual machine from the given program which will capture
/// all output into a buffer, which can be retrieved from
/// `rune_tests::capture_output::drain_output()`
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
/// use rune::Value;
///
/// let mut vm = rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// ```
macro_rules! rune_vm_capture {
    ($($tt:tt)*) => {{
        let mut context = rune::Context::with_config(false)?;
        let io = $crate::capture_io::CaptureIo::new();
        let m = $crate::capture_io::module(&io)?;
        context.install(m)?;
        let mut sources = $crate::sources(stringify!($($tt)*));
        let mut diagnostics = Default::default();
        let vm = $crate::vm(&context, &mut sources, &mut diagnostics)?;
        (vm, io)
    }};
}

mod benchmarks;

criterion::criterion_main! {
    benchmarks::aoc_2020_1a::benches,
    benchmarks::aoc_2020_1b::benches,
    benchmarks::aoc_2020_11a::benches,
    benchmarks::aoc_2020_19b::benches,
    benchmarks::brainfuck::benches,
    benchmarks::fib::benches,
}
