use rune::{BuildError, Context, Diagnostics, Options, Source, Sources, Vm};
use std::any::Any;
use std::sync::Arc;

pub(crate) fn vm(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
) -> Result<Vm, BuildError> {
    let mut options = Options::from_default_env().expect("failed to build options");

    options
        .parse_option("function-body=true")
        .expect("failed to parse option");

    let unit = rune::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .with_options(&options)
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

macro_rules! rhai_ast {
    ($level:ident { $($tt:tt)* }) => {{
        let mut engine = $crate::rhai::Engine::new();
        engine.set_optimization_level($crate::rhai::OptimizationLevel::$level);
        let ast = engine.compile(stringify!($($tt)*)).unwrap();
        $crate::RhaiRunner { engine, ast }
    }};
}

pub(crate) struct RhaiRunner {
    pub(crate) engine: crate::rhai::Engine,
    pub(crate) ast: crate::rhai::AST,
}

impl RhaiRunner {
    fn eval<T: Any + Clone>(&self) -> T {
        self.engine.eval_ast(&self.ast).unwrap()
    }
}

pub(crate) mod rhai {
    pub(crate) use ::rhai::{Engine, OptimizationLevel, AST};
}

mod comparisons {
    pub mod eval;
    pub mod primes;
}

criterion::criterion_main! {
    comparisons::primes::benches,
    comparisons::eval::benches,
}
