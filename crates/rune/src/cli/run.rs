use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use clap::Parser;

use crate::cli::{AssetKind, CommandBase, Config, ExitCode, Io, SharedFlags};
use crate::runtime::{UnitStorage, VmError, VmExecution, VmResult};
use crate::{Context, Sources, Unit, Value, Vm};

#[derive(Parser, Debug)]
pub(super) struct Flags {
    /// Provide detailed tracing for each instruction executed.
    #[arg(short, long)]
    trace: bool,
    /// When tracing is enabled, do not include source references if they are
    /// available.
    #[arg(long)]
    without_source: bool,
    /// Time how long the script took to execute.
    #[arg(long)]
    time: bool,
    /// Perform a default dump.
    #[arg(short, long)]
    dump: bool,
    /// Dump return value.
    #[arg(long)]
    dump_return: bool,
    /// Dump everything that is available, this is very verbose.
    #[arg(long)]
    dump_all: bool,
    /// Dump default information about unit.
    #[arg(long)]
    dump_unit: bool,
    /// Dump constants from the unit.
    #[arg(long)]
    dump_constants: bool,
    /// Dump unit instructions.
    #[arg(long)]
    emit_instructions: bool,
    /// Dump the state of the stack after completion.
    ///
    /// If compiled with `--trace` will dump it after each instruction.
    #[arg(long)]
    dump_stack: bool,
    /// Dump dynamic functions.
    #[arg(long)]
    dump_functions: bool,
    /// Dump dynamic types.
    #[arg(long)]
    dump_types: bool,
    /// Dump native functions.
    #[arg(long)]
    dump_native_functions: bool,
    /// Dump native types.
    #[arg(long)]
    dump_native_types: bool,
    /// When tracing, limit the number of instructions to run with `limit`. This
    /// implies `--trace`.
    #[arg(long)]
    trace_limit: Option<usize>,
}

impl CommandBase for Flags {
    #[inline]
    fn is_workspace(&self, kind: AssetKind) -> bool {
        matches!(kind, AssetKind::Bin)
    }

    #[inline]
    fn propagate(&mut self, _: &mut Config, _: &mut SharedFlags) {
        if self.dump || self.dump_all {
            self.dump_unit = true;
            self.dump_stack = true;
            self.dump_return = true;
        }

        if self.dump_all {
            self.dump_constants = true;
            self.dump_functions = true;
            self.dump_types = true;
            self.dump_native_functions = true;
            self.dump_native_types = true;
        }

        if self.dump_functions
        || self.dump_native_functions
        || self.dump_stack
        || self.dump_types
        || self.dump_constants {
            self.dump_unit = true;
        }

        if self.dump_unit {
            self.emit_instructions = true;
        }

        if self.trace_limit.is_some() {
            self.trace = true;
        }
    }
}

enum TraceError {
    Io(std::io::Error),
    VmError(VmError),
    Limited,
}

impl From<std::io::Error> for TraceError {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<VmError> for TraceError {
    #[inline]
    fn from(error: VmError) -> Self {
        Self::VmError(error)
    }
}

pub(super) async fn run(
    io: &mut Io<'_>,
    c: &Config,
    args: &Flags,
    context: &Context,
    unit: Arc<Unit>,
    sources: &Sources,
) -> Result<ExitCode> {
    if args.dump_native_functions {
        writeln!(io.stdout, "# functions")?;

        for (i, (meta, _)) in context.iter_functions().enumerate() {
            if let Some(item) = &meta.item {
                writeln!(io.stdout, "{:04} = {} ({})", i, item, meta.hash)?;
            }
        }
    }

    if args.dump_native_types {
        writeln!(io.stdout, "# types")?;

        for (i, (hash, ty)) in context.iter_types().enumerate() {
            writeln!(io.stdout, "{:04} = {} ({})", i, ty, hash)?;
        }
    }

    if args.dump_unit {
        writeln!(
            io.stdout,
            "Unit size: {} bytes",
            unit.instructions().bytes()
        )?;

        if args.emit_instructions {
            let mut o = io.stdout.lock();
            writeln!(o, "# instructions")?;
            unit.emit_instructions(&mut o, sources, args.without_source)?;
        }

        let mut functions = unit.iter_functions().peekable();
        let mut strings = unit.iter_static_strings().peekable();
        let mut keys = unit.iter_static_object_keys().peekable();
        let mut constants = unit.iter_constants().peekable();

        if args.dump_functions && functions.peek().is_some() {
            writeln!(io.stdout, "# dynamic functions")?;

            for (hash, kind) in functions {
                if let Some(signature) = unit.debug_info().and_then(|d| d.functions.get(&hash)) {
                    writeln!(io.stdout, "{} = {}", hash, signature)?;
                } else {
                    writeln!(io.stdout, "{} = {}", hash, kind)?;
                }
            }
        }

        if strings.peek().is_some() {
            writeln!(io.stdout, "# strings")?;

            for string in strings {
                writeln!(io.stdout, "{} = {:?}", string.hash(), string)?;
            }
        }

        if args.dump_constants && constants.peek().is_some() {
            writeln!(io.stdout, "# constants")?;

            for constant in constants {
                writeln!(io.stdout, "{} = {:?}", constant.0, constant.1)?;
            }
        }

        if keys.peek().is_some() {
            writeln!(io.stdout, "# object keys")?;

            for (hash, keys) in keys {
                writeln!(io.stdout, "{} = {:?}", hash, keys)?;
            }
        }
    }

    let runtime = Arc::new(context.runtime()?);

    let last = Instant::now();

    let mut vm = Vm::new(runtime, unit);
    let mut execution: VmExecution<_> = vm.execute(["main"], ())?;

    let result = if args.trace {
        match do_trace(
            io,
            &mut execution,
            sources,
            args.dump_stack,
            args.without_source,
            args.trace_limit.unwrap_or(usize::MAX),
        )
        .await
        {
            Ok(value) => VmResult::Ok(value),
            Err(TraceError::Io(io)) => return Err(io.into()),
            Err(TraceError::VmError(vm)) => VmResult::Err(vm),
            Err(TraceError::Limited) => return Err(anyhow!("Trace limit reached")),
        }
    } else {
        execution.async_complete().await
    };

    let errored = match result {
        VmResult::Ok(result) => {
            if c.verbose || args.time || args.dump_return {
                let duration = Instant::now().saturating_duration_since(last);
                writeln!(io.stderr, "== {:?} ({:?})", result, duration)?;
            }

            None
        }
        VmResult::Err(error) => {
            if c.verbose || args.time || args.dump_return {
                let duration = Instant::now().saturating_duration_since(last);
                writeln!(io.stderr, "== ! ({}) ({:?})", error, duration)?;
            }

            Some(error)
        }
    };

    if args.dump_stack {
        writeln!(io.stdout, "# full stack dump after halting")?;

        let vm = execution.vm();

        let frames = vm.call_frames();
        let stack = vm.stack();

        let mut it = frames.iter().enumerate().peekable();

        while let Some((count, frame)) = it.next() {
            let stack_top = match it.peek() {
                Some((_, next)) => next.top,
                None => stack.top(),
            };

            let values = stack
                .get(frame.top..stack_top)
                .expect("bad stack slice");

            writeln!(io.stdout, "  frame #{} (+{})", count, frame.top)?;

            if values.is_empty() {
                writeln!(io.stdout, "    *empty*")?;
            }

            vm.with(|| {
                for (n, value) in stack.iter().enumerate() {
                    writeln!(io.stdout, "    {}+{n} = {value:?}", frame.top)?;
                }

                Ok::<_, crate::support::Error>(())
            })?;
        }

        // NB: print final frame
        writeln!(
            io.stdout,
            "  frame #{} (+{})",
            frames.len(),
            stack.top()
        )?;

        let values = stack.get(stack.top()..).expect("bad stack slice");

        if values.is_empty() {
            writeln!(io.stdout, "    *empty*")?;
        }

        vm.with(|| {
            for (n, value) in values.iter().enumerate() {
                writeln!(io.stdout, "    {}+{n} = {value:?}", stack.top())?;
            }

            Ok::<_, crate::support::Error>(())
        })?;
    }

    if let Some(error) = errored {
        error.emit(io.stdout, sources)?;
        Ok(ExitCode::VmError)
    } else {
        Ok(ExitCode::Success)
    }
}

/// Perform a detailed trace of the program.
async fn do_trace<T>(
    io: &Io<'_>,
    execution: &mut VmExecution<T>,
    sources: &Sources,
    dump_stack: bool,
    without_source: bool,
    mut limit: usize,
) -> Result<Value, TraceError>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    let mut current_frame_len = execution.vm().call_frames().len();
    let mut result = VmResult::Ok(None);

    while limit > 0 {
        let vm = execution.vm();
        let ip = vm.ip();
        let mut o = io.stdout.lock();

        if let Some((hash, signature)) = vm
            .unit()
            .debug_info()
            .and_then(|d| d.function_at(ip))
        {
            writeln!(o, "fn {} ({}):", signature, hash)?;
        }

        let debug = vm
            .unit()
            .debug_info()
            .and_then(|d| d.instruction_at(ip));

        for label in debug.map(|d| d.labels.as_slice()).unwrap_or_default() {
            writeln!(o, "{}:", label)?;
        }

        if !without_source {
            let debug_info = debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)));

            if let Some(line) = debug_info.and_then(|(s, span)| s.source_line(span)) {
                write!(o, "  ")?;
                line.write(&mut o)?;
                writeln!(o)?;
            }
        }

        if dump_stack {
            let frames = vm.call_frames();
            let stack = vm.stack();

            if current_frame_len != frames.len() {
                let op = if current_frame_len < frames.len() { "push" } else { "pop" };
                write!(o, "  {op} frame {} (+{})", frames.len(), stack.top())?;

                if let Some(frame) = frames.last() {
                    writeln!(o, " {frame:?}")?;
                } else {
                    writeln!(o, " *root*")?;
                }

                current_frame_len = frames.len();
            }
        }

        if let Some((inst, _)) = vm
            .unit()
            .instruction_at(ip)
            .map_err(VmError::from)?
        {
            write!(o, "  {:04} = {}", ip, inst)?;
        } else {
            write!(o, "  {:04} = *out of bounds*", ip)?;
        }

        if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
            write!(o, " // {}", comment)?;
        }

        writeln!(o)?;

        if dump_stack {
            let stack = vm.stack();
            let values = stack.get(stack.top()..).expect("bad stack slice");

            vm.with(|| {
                for (n, value) in values.iter().enumerate() {
                    writeln!(o, "    {}+{n} = {value:?}", stack.top())?;
                }

                Ok::<_, TraceError>(())
            })?;
        }

        match result {
            VmResult::Ok(result) => {
                if let Some(result) = result {
                    return Ok(result);
                }
            }
            VmResult::Err(error) => {
                return Err(TraceError::VmError(error));
            }
        }

        result = execution.async_step().await;
        limit = limit.wrapping_sub(1);
    }

    Err(TraceError::Limited)
}
