use std::fmt;
use std::io::Write;
use std::slice;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::cli::naming::Naming;
use crate::cli::visitor;
use crate::cli::{
    AssetKind, CommandBase, Config, Entry, EntryPoint, ExitCode, Io, Options, SharedFlags,
};
use crate::compile::FileSourceLoader;
use crate::doc::TestParams;
use crate::item::ComponentRef;
use crate::modules::capture_io::CaptureIo;
use crate::runtime::{Value, ValueKind, Vm, VmError, VmResult};
use crate::termcolor::{Color, ColorSpec, WriteColor};
use crate::{Diagnostics, Hash, Item, ItemBuf, Source, Sources, Unit};

mod cli {
    use std::string::String;
    use std::vec::Vec;

    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    #[command(rename_all = "kebab-case")]
    pub struct Flags {
        /// Exit with a non-zero exit-code even for warnings
        #[arg(long)]
        pub warnings_are_errors: bool,
        /// Display one character per test instead of one line
        #[arg(long, short = 'q')]
        pub quiet: bool,
        /// Break on the first test failed.
        #[arg(long)]
        pub fail_fast: bool,
        /// Filter tests by name.
        pub filters: Vec<String>,
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

enum BatchKind {
    LibTests,
    DocTests,
    ContextDocTests,
}

impl fmt::Display for BatchKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibTests => write!(f, "lib tests"),
            Self::DocTests => write!(f, "doc tests"),
            Self::ContextDocTests => write!(f, "context doc tests"),
        }
    }
}

struct Batch<'a> {
    kind: BatchKind,
    entry: Option<EntryPoint<'a>>,
    cases: Vec<TestCase>,
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

    let mut executed = 0usize;
    let mut skipped = 0usize;
    let mut build_errors = 0usize;

    let capture = crate::modules::capture_io::CaptureIo::new();
    let context = shared.context(entry, c, Some(&capture))?;

    let mut batches = Vec::new();
    let mut naming = Naming::default();
    let mut name = String::new();

    let mut filter = |item: &Item| -> Result<bool> {
        if flags.filters.is_empty() {
            return Ok(false);
        }

        name.clear();

        write!(name, "{item}")?;

        if !flags.filters.iter().any(|f| name.contains(f.as_str())) {
            return Ok(true);
        }

        Ok(false)
    };

    for e in entries {
        let mut options = options.clone();

        if e.is_argument() {
            options.function_body = true;
        }

        let item = naming.item(&e)?;

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
            .with_options(&options)
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

        let mut cases = Vec::new();

        for (hash, item) in functions.into_functions() {
            let filtered = filter(&item)?;

            cases.try_push(TestCase::new(
                hash,
                item,
                unit.clone(),
                sources.clone(),
                TestParams::default(),
                filtered,
            ))?;
        }

        batches.try_push(Batch {
            kind: BatchKind::LibTests,
            entry: Some(e.try_clone()?),
            cases,
        })?;

        let mut artifacts = crate::doc::Artifacts::without_assets();

        crate::doc::build("root", &mut artifacts, None, slice::from_ref(&doc_visitor))?;

        let cases = populate_doc_tests(
            io,
            artifacts,
            shared,
            flags,
            &options,
            &context,
            &mut build_errors,
            &mut filter,
        )?;

        batches.try_push(Batch {
            kind: BatchKind::DocTests,
            entry: Some(e),
            cases,
        })?;
    }

    let mut artifacts = crate::doc::Artifacts::without_assets();
    crate::doc::build("root", &mut artifacts, Some(&context), &[])?;

    let cases = populate_doc_tests(
        io,
        artifacts,
        shared,
        flags,
        options,
        &context,
        &mut build_errors,
        &mut filter,
    )?;

    batches.try_push(Batch {
        kind: BatchKind::ContextDocTests,
        entry: None,
        cases,
    })?;

    let runtime = Arc::new(context.runtime()?);
    let mut failed = Vec::new();

    for batch in batches {
        if batch.cases.is_empty() {
            continue;
        }

        if !flags.quiet {
            let all_ignored = batch
                .cases
                .iter()
                .all(|case| case.filtered || case.params.no_run);

            if all_ignored {
                io.stdout.set_color(&colors.ignored)?;
                write!(io.stdout, "{:>12}", "Ignoring")?;
                io.stdout.reset()?;
            } else {
                io.stdout.set_color(&colors.highlight)?;
                write!(io.stdout, "{:>12}", "Running")?;
                io.stdout.reset()?;
            }

            write!(io.stdout, " {} {}", batch.cases.len(), batch.kind)?;

            if let Some(entry) = batch.entry {
                write!(io.stdout, " from {entry}")?;
            }

            writeln!(io.stdout)?;
        }

        for mut case in batch.cases {
            if case.filtered || case.params.no_run {
                skipped = skipped.wrapping_add(1);
                continue;
            }

            let mut vm = Vm::new(runtime.clone(), case.unit.clone());
            case.execute(&mut vm, &capture).await?;
            executed = executed.wrapping_add(1);

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
    }

    if flags.quiet {
        writeln!(io.stdout)?;
    }

    let failures = failed.len();

    for case in failed {
        case.emit(io, &colors)?;
    }

    let elapsed = start.elapsed();

    io.stdout.set_color(&colors.highlight)?;
    write!(io.stdout, "{:>12}", "Executed")?;
    io.stdout.reset()?;

    writeln!(
        io.stdout,
        " {executed} tests with {failures} failures \
        ({skipped} skipped, {build_errors} build errors) in {:.3} seconds",
        elapsed.as_secs_f64()
    )?;

    if build_errors == 0 && failures == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}

fn populate_doc_tests(
    io: &mut Io,
    artifacts: crate::doc::Artifacts,
    shared: &SharedFlags,
    flags: &Flags,
    options: &Options,
    context: &crate::Context,
    build_errors: &mut usize,
    filter: &mut dyn FnMut(&Item) -> Result<bool>,
) -> Result<Vec<TestCase>> {
    let mut cases = Vec::new();

    for test in artifacts.tests() {
        if !options.test_std && test.item.as_crate() == Some("std") || test.params.ignore {
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
            .with_context(context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            *build_errors = build_errors.wrapping_add(1);
            continue;
        }

        if !test.params.no_run {
            let unit = Arc::new(unit?);
            let sources = Arc::new(sources);

            cases.try_push(TestCase::new(
                Hash::type_hash([ComponentRef::Id(0)]),
                test.item.try_clone()?,
                unit.clone(),
                sources.clone(),
                test.params,
                filter(&test.item)?,
            ))?;
        }
    }
    Ok(cases)
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
    filtered: bool,
}

impl TestCase {
    fn new(
        hash: Hash,
        item: ItemBuf,
        unit: Arc<Unit>,
        sources: Arc<Sources>,
        params: TestParams,
        filtered: bool,
    ) -> Self {
        Self {
            hash,
            item,
            unit,
            sources,
            params,
            outcome: Outcome::Ok,
            output: Vec::new(),
            filtered,
        }
    }

    async fn execute(&mut self, vm: &mut Vm, capture_io: &CaptureIo) -> Result<()> {
        let result = match vm.execute(self.hash, ()) {
            Ok(mut execution) => execution.async_complete().await,
            Err(err) => VmResult::Err(err),
        };

        capture_io.drain_into(&mut self.output)?;

        self.outcome = match result {
            VmResult::Ok(v) => match v.take_kind()? {
                ValueKind::Result(result) => match result {
                    Ok(..) => Outcome::Ok,
                    Err(error) => Outcome::Err(error),
                },
                ValueKind::Option(option) => match option {
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
        io.stdout.set_color(&colors.highlight)?;
        write!(io.stdout, "{:>12}", "Test")?;
        io.stdout.reset()?;

        write!(io.stdout, " {}: ", self.item)?;

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
    highlight: ColorSpec,
    ignored: ColorSpec,
}

impl Colors {
    fn new() -> Self {
        let mut this = Self {
            error: ColorSpec::new(),
            passed: ColorSpec::new(),
            highlight: ColorSpec::new(),
            ignored: ColorSpec::new(),
        };

        this.error.set_fg(Some(Color::Red));
        this.passed.set_fg(Some(Color::Green));
        this.highlight.set_fg(Some(Color::Green));
        this.highlight.set_bold(true);
        this.ignored.set_fg(Some(Color::Yellow));
        this.ignored.set_bold(true);
        this
    }
}
