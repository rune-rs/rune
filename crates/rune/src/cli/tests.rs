use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::cli::naming::Naming;
use crate::cli::visitor;
use crate::cli::{
    AssetKind, CommandBase, Config, Entry, EntryPoint, ExitCode, Io, Options, SharedFlags,
};
use crate::compile::{FileSourceLoader, ItemBuf};
use crate::doc::TestParams;
use crate::modules::capture_io::CaptureIo;
use crate::runtime::{UnitFn, Value, Vm, VmError, VmResult};
use crate::termcolor::{Color, ColorSpec, WriteColor};
use crate::{Diagnostics, Hash, Source, Sources, Unit};

mod cli {
    use ::rust_alloc::string::String;
    use ::rust_alloc::vec::Vec;
    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    pub struct Flags {
        /// Exit with a non-zero exit-code even for warnings
        #[arg(long)]
        pub warnings_are_errors: bool,
        /// Display one character per test instead of one line
        #[arg(long, short = 'q')]
        pub quiet: bool,
        /// Also run tests for `::std`.
        #[arg(long, long = "opt")]
        pub options: Vec<String>,
        /// Break on the first test failed.
        #[arg(long)]
        pub fail_fast: bool,
    }
}

pub(super) use cli::Flags;

impl CommandBase for Flags {
    #[inline]
    fn is_debug(&self) -> bool {
        true
    }

    #[inline]
    fn is_workspace(&self, kind: AssetKind) -> bool {
        matches!(kind, AssetKind::Test)
    }

    #[inline]
    fn describe(&self) -> &str {
        "Testing"
    }

    #[inline]
    fn propagate(&mut self, c: &mut Config, _: &mut SharedFlags) {
        c.test = true;
    }
}

/// Run all tests that can be found.
pub(super) async fn run<'p, I>(
    io: &mut Io<'_>,
    c: &Config,
    flags: &Flags,
    shared: &SharedFlags,
    options: &Options,
    entry: &mut Entry<'_>,
    entries: I,
) -> anyhow::Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'p>>,
{
    let colors = Colors::new();

    let start = Instant::now();

    let mut build_errors = 0usize;
    let mut executed = 0usize;

    let capture = crate::modules::capture_io::CaptureIo::new();
    let context = shared.context(entry, c, Some(&capture))?;

    let mut doc_visitors = Vec::new();
    let mut cases = Vec::new();
    let mut naming = Naming::default();

    let mut include_std = false;

    for opt in &flags.options {
        match opt.as_str() {
            "include-std" => {
                include_std = true;
            }
            other => {
                bail!("Unsupported option: {other}")
            }
        }
    }

    for e in entries {
        let name = naming.name(&e)?;
        let item = ItemBuf::with_crate(&name)?;

        let mut sources = Sources::new();

        let source = match Source::from_path(e.path()) {
            Ok(source) => source,
            Err(error) => return Err(error).context(e.path().display().try_to_string()?),
        };

        sources.insert(source)?;

        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut doc_visitor = crate::doc::Visitor::new(&item)?;
        let mut functions = visitor::FunctionVisitor::new(visitor::Attribute::Test);
        let mut source_loader = FileSourceLoader::new();

        let unit = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(options)
            .with_visitor(&mut doc_visitor)?
            .with_visitor(&mut functions)?
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            build_errors = build_errors.wrapping_add(1);
            continue;
        }

        let unit = Arc::new(unit?);
        let sources = Arc::new(sources);

        doc_visitors.try_push(doc_visitor)?;

        for (hash, item) in functions.into_functions() {
            cases.try_push(TestCase::new(
                hash,
                item,
                unit.clone(),
                sources.clone(),
                TestParams::default(),
            ))?;
        }
    }

    let mut artifacts = crate::doc::Artifacts::without_assets();
    crate::doc::build("root", &mut artifacts, &context, &doc_visitors)?;

    for test in artifacts.tests() {
        if test.item.as_crate() == Some("std") && !include_std || test.params.ignore {
            continue;
        }

        let mut sources = Sources::new();

        let source = Source::new(test.item.try_to_string()?, &test.content)?;
        sources.insert(source)?;

        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut source_loader = FileSourceLoader::new();

        let mut options = options.clone();
        options.function_body = true;

        let unit = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            build_errors = build_errors.wrapping_add(1);
            continue;
        }

        if !test.params.no_run {
            let unit = Arc::new(unit?);
            let sources = Arc::new(sources);

            let Some((hash, _)) = unit.iter_functions().find(|(_, f)| {
                matches!(
                    f,
                    UnitFn::Offset {
                        args: 0,
                        offset: 0,
                        ..
                    }
                )
            }) else {
                bail!("Compiling source did not result in a function at offset 0");
            };

            cases.try_push(TestCase::new(
                hash,
                test.item.try_clone()?,
                unit.clone(),
                sources.clone(),
                test.params,
            ))?;
        }
    }

    let runtime = Arc::new(context.runtime()?);
    let mut failed = Vec::new();

    let total = cases.len();

    for mut case in cases {
        executed = executed.wrapping_add(1);

        let mut vm = Vm::new(runtime.clone(), case.unit.clone());
        case.execute(&mut vm, &capture).await?;

        if case.outcome.is_ok() {
            if flags.quiet {
                write!(io.stdout, ".")?;
            } else {
                case.emit(io, &colors)?;
            }

            continue;
        }

        if flags.quiet {
            write!(io.stdout, "f")?;
        }

        failed.try_push(case)?;

        if flags.fail_fast {
            break;
        }
    }

    if flags.quiet {
        writeln!(io.stdout)?;
    }

    let failures = failed.len();

    for case in failed {
        case.emit(io, &colors)?;
    }

    let elapsed = start.elapsed();

    writeln!(
        io.stdout,
        "Executed {} tests with {} failures ({} skipped, {} build errors) in {:.3} seconds",
        executed,
        failures,
        total - executed,
        build_errors,
        elapsed.as_secs_f64()
    )?;

    if build_errors == 0 && failures == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}

#[derive(Debug)]
enum Outcome {
    Ok,
    Panic(VmError),
    ExpectedPanic,
    None,
    Err(Value),
}

impl Outcome {
    fn is_ok(&self) -> bool {
        matches!(self, Outcome::Ok)
    }
}

struct TestCase {
    hash: Hash,
    item: ItemBuf,
    unit: Arc<Unit>,
    sources: Arc<Sources>,
    params: TestParams,
    outcome: Outcome,
    output: Vec<u8>,
}

impl TestCase {
    fn new(
        hash: Hash,
        item: ItemBuf,
        unit: Arc<Unit>,
        sources: Arc<Sources>,
        params: TestParams,
    ) -> Self {
        Self {
            hash,
            item,
            unit,
            sources,
            params,
            outcome: Outcome::Ok,
            output: Vec::new(),
        }
    }

    async fn execute(&mut self, vm: &mut Vm, capture_io: &CaptureIo) -> Result<()> {
        let result = match vm.execute(self.hash, ()) {
            Ok(mut execution) => execution.async_complete().await,
            Err(err) => VmResult::Err(err),
        };

        capture_io.drain_into(&mut self.output)?;

        self.outcome = match result {
            VmResult::Ok(v) => match v {
                Value::Result(result) => match result.take()? {
                    Ok(..) => Outcome::Ok,
                    Err(error) => Outcome::Err(error),
                },
                Value::Option(option) => match *option.borrow_ref()? {
                    Some(..) => Outcome::Ok,
                    None => Outcome::None,
                },
                _ => Outcome::Ok,
            },
            VmResult::Err(e) => Outcome::Panic(e),
        };

        if self.params.should_panic {
            if matches!(self.outcome, Outcome::Panic(..)) {
                self.outcome = Outcome::Ok;
            } else {
                self.outcome = Outcome::ExpectedPanic;
            }
        }

        Ok(())
    }

    fn emit(self, io: &mut Io<'_>, colors: &Colors) -> Result<()> {
        write!(io.stdout, "Test {}: ", self.item)?;

        match &self.outcome {
            Outcome::Panic(error) => {
                io.stdout.set_color(&colors.error)?;
                writeln!(io.stdout, "panicked")?;
                io.stdout.reset()?;

                error.emit(io.stdout, &self.sources)?;
            }
            Outcome::ExpectedPanic => {
                io.stdout.set_color(&colors.error)?;
                writeln!(
                    io.stdout,
                    "expected panic because of `should_panic`, but ran without issue"
                )?;
                io.stdout.reset()?;
            }
            Outcome::Err(error) => {
                io.stdout.set_color(&colors.error)?;
                write!(io.stdout, "err: ")?;
                io.stdout.reset()?;
                writeln!(io.stdout, "{:?}", error)?;
            }
            Outcome::None => {
                io.stdout.set_color(&colors.error)?;
                writeln!(io.stdout, "returned none")?;
                io.stdout.reset()?;
            }
            Outcome::Ok => {
                io.stdout.set_color(&colors.passed)?;
                writeln!(io.stdout, "ok")?;
                io.stdout.reset()?;
            }
        }

        if !self.outcome.is_ok() && !self.output.is_empty() {
            writeln!(io.stdout, "-- output --")?;
            io.stdout.write_all(&self.output)?;
            writeln!(io.stdout, "-- end of output --")?;
        }

        Ok(())
    }
}

struct Colors {
    error: ColorSpec,
    passed: ColorSpec,
}

impl Colors {
    fn new() -> Self {
        let mut this = Self {
            error: ColorSpec::new(),
            passed: ColorSpec::new(),
        };

        this.error.set_fg(Some(Color::Red));
        this.passed.set_fg(Some(Color::Green));
        this
    }
}
