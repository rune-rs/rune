use rune::{BuildError, Context, Diagnostics, Source, Sources, Vm};
use std::sync::Arc;

pub(crate) fn vm(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
) -> Result<Vm, BuildError> {
    let unit = rune::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .build()?;

    let context = Arc::new(context.runtime()?);
    Ok(Vm::new(context, Arc::new(unit)))
}

pub(crate) fn sources(source: &str) -> Sources {
    let mut sources = Sources::new();
    sources
        .insert(Source::new("main", source).expect("Failed to construct source"))
        .expect("Failed to insert source");
    sources
}

macro_rules! rune_vm {
    ($($tt:tt)*) => {{
        let context = rune::Context::with_default_modules().expect("Failed to build context");
        let mut diagnostics = Default::default();
        let mut sources = $crate::sources(stringify!($($tt)*));
        $crate::vm(&context, &mut sources, &mut diagnostics).expect("Program to compile successfully")
    }};
}

macro_rules! rune_vm_capture {
    ($($tt:tt)*) => {{
        let mut context = rune::Context::with_config(false)?;
        let io = rune::modules::capture_io::CaptureIo::new();
        let m = rune::modules::capture_io::module(&io)?;
        context.install(m)?;
        let mut sources = $crate::sources(stringify!($($tt)*));
        let mut diagnostics = Default::default();
        let vm = $crate::vm(&context, &mut sources, &mut diagnostics)?;
        (vm, io)
    }};
}

mod benchmarks {
    pub mod aoc_2020_11a;
    pub mod aoc_2020_19b;
    pub mod aoc_2020_1a;
    pub mod aoc_2020_1b;
    pub mod brainfuck;
    pub mod external_functions;
    pub mod fib;
}

criterion::criterion_main! {
    benchmarks::aoc_2020_1a::benches,
    benchmarks::aoc_2020_1b::benches,
    benchmarks::aoc_2020_11a::benches,
    benchmarks::aoc_2020_19b::benches,
    benchmarks::brainfuck::benches,
    benchmarks::fib::benches,
    benchmarks::external_functions::benches,
}
