use wasm_bindgen::prelude::*;

use rune::{EmitDiagnostics as _, Spanned as _};
use runestick::budget;
use runestick::{ContextError, Module, Panic, Stack, Value, VmError};
use serde::Serialize;
use std::cell;
use std::fmt;
use std::io;
use std::sync::Arc;

#[derive(Default, Serialize)]
struct Position {
    line: u32,
    character: u32,
}

impl From<(usize, usize)> for Position {
    fn from((line, character): (usize, usize)) -> Self {
        Self {
            line: line as u32,
            character: character as u32,
        }
    }
}

#[derive(Serialize)]
enum DiagnosticKind {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
}

#[derive(Serialize)]
struct Diagnostic {
    kind: DiagnosticKind,
    start: Position,
    end: Position,
    message: String,
}

#[derive(Serialize)]
pub struct CompileResult {
    error: Option<String>,
    diagnostics_output: Option<String>,
    diagnostics: Vec<Diagnostic>,
    result: Option<String>,
    output: Option<String>,
}

impl CompileResult {
    /// Construct output from compile result.
    fn output(
        output: Value,
        diagnostics_output: Option<String>,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            error: None,
            diagnostics_output,
            diagnostics,
            result: Some(format!("{:?}", output)),
            output: drain_output(),
        }
    }

    /// Construct a result from an error.
    fn from_error<E>(
        error: E,
        diagnostics_output: Option<String>,
        diagnostics: Vec<Diagnostic>,
    ) -> Self
    where
        E: fmt::Display,
    {
        Self {
            error: Some(error.to_string()),
            diagnostics_output,
            diagnostics,
            result: None,
            output: drain_output(),
        }
    }
}

fn inner_compile(input: &str, budget: usize) -> CompileResult {
    let source = runestick::Source::new("entry", input);
    let mut sources = rune::Sources::new();
    sources.insert(source);

    let mut context = match runestick::Context::with_config(false) {
        Ok(context) => context,
        Err(error) => {
            return CompileResult::from_error(error, None, Vec::new());
        }
    };

    let module = match wasm_module() {
        Ok(module) => module,
        Err(error) => {
            return CompileResult::from_error(
                format!("Failed to setup wasm module: {}", error),
                None,
                Vec::new(),
            );
        }
    };

    if let Err(error) = context.install(&module) {
        return CompileResult::from_error(
            format!("Failed to install WASM module: {}", error),
            None,
            Vec::new(),
        );
    }

    let context = Arc::new(context);
    let options = rune::Options::default();
    let mut errors = rune::Errors::new();
    let mut warnings = rune::Warnings::new();

    let mut diagnostics = Vec::new();

    let result = rune::load_sources(
        &*context,
        &options,
        &mut sources,
        &mut errors,
        &mut warnings,
    );

    for warning in &warnings {
        let span = warning.span();

        if let Some(source) = sources.get(warning.source_id) {
            let start = Position::from(source.position_to_unicode_line_char(span.start));
            let end = Position::from(source.position_to_unicode_line_char(span.end));

            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Warning,
                start,
                end,
                message: warning.to_string(),
            });
        }
    }

    let mut writer = rune::termcolor::Buffer::no_color();

    warnings
        .emit_diagnostics(&mut writer, &sources)
        .expect("emitting to buffer should never fail");

    let unit = match result {
        Ok(unit) => Arc::new(unit),
        Err(error) => {
            for error in &errors {
                if let Some(source) = sources.get(error.source_id()) {
                    match error.kind() {
                        rune::LoadErrorKind::ParseError(error) => {
                            let span = error.span();

                            let start =
                                Position::from(source.position_to_unicode_line_char(span.start));
                            let end =
                                Position::from(source.position_to_unicode_line_char(span.end));

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: error.to_string(),
                            });
                        }
                        rune::LoadErrorKind::CompileError(error) => {
                            let span = error.span();

                            let start =
                                Position::from(source.position_to_unicode_line_char(span.start));
                            let end =
                                Position::from(source.position_to_unicode_line_char(span.end));

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: error.to_string(),
                            });
                        }
                        rune::LoadErrorKind::LinkError(error) => match error {
                            rune::LinkerError::MissingFunction { hash, spans } => {
                                for (span, _) in spans {
                                    let start = Position::from(
                                        source.position_to_unicode_line_char(span.start),
                                    );
                                    let end = Position::from(
                                        source.position_to_unicode_line_char(span.end),
                                    );

                                    diagnostics.push(Diagnostic {
                                        kind: DiagnosticKind::Error,
                                        start,
                                        end,
                                        message: format!("missing function (hash: {})", hash),
                                    });
                                }
                            }
                        },
                        rune::LoadErrorKind::Internal(_) => {}
                    }
                }
            }

            errors
                .emit_diagnostics(&mut writer, &sources)
                .expect("emitting to buffer should never fail");

            return CompileResult::from_error(error, diagnostics_output(writer), diagnostics);
        }
    };

    let vm = runestick::Vm::new(context, unit);

    let mut execution = match vm.execute(&["main"], ()) {
        Ok(execution) => execution,
        Err(error) => {
            error
                .emit_diagnostics(&mut writer, &sources)
                .expect("emitting to buffer should never fail");

            return CompileResult::from_error(error, diagnostics_output(writer), diagnostics);
        }
    };

    let future = budget::with(budget, execution.async_complete());

    let output = match futures_executor::block_on(future) {
        Ok(output) => output,
        Err(error) => {
            if let Ok(vm) = execution.vm() {
                let (kind, unwound) = error.as_unwound();

                let (unit, ip) = match unwound {
                    Some((unit, ip)) => (unit, ip),
                    None => (vm.unit(), vm.ip()),
                };

                // NB: emit diagnostics if debug info is available.
                if let Some(debug) = unit.debug_info() {
                    if let Some(inst) = debug.instruction_at(ip) {
                        if let Some(source) = sources.get(inst.source_id) {
                            let start = Position::from(
                                source.position_to_unicode_line_char(inst.span.start),
                            );
                            let end =
                                Position::from(source.position_to_unicode_line_char(inst.span.end));

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: kind.to_string(),
                            });
                        }
                    }
                }
            }

            error
                .emit_diagnostics(&mut writer, &sources)
                .expect("emitting to buffer should never fail");

            return CompileResult::from_error(error, diagnostics_output(writer), diagnostics);
        }
    };

    CompileResult::output(output, diagnostics_output(writer), diagnostics)
}

fn diagnostics_output(writer: rune::termcolor::Buffer) -> Option<String> {
    let mut string = String::from_utf8(writer.into_inner()).ok()?;
    let new_len = string.trim_end().len();
    string.truncate(new_len);
    Some(string)
}

#[wasm_bindgen]
pub fn compile(input: &str, budget: usize) -> JsValue {
    JsValue::from_serde(&inner_compile(input, budget)).unwrap()
}

thread_local!(static OUT: cell::RefCell<io::Cursor<Vec<u8>>> = cell::RefCell::new(io::Cursor::new(Vec::new())));

/// Drain all output that has been written to `OUT`. If `OUT` contains non -
/// UTF-8, will drain but will still return `None`.
fn drain_output() -> Option<String> {
    OUT.with(|out| {
        let mut out = out.borrow_mut();
        let out = std::mem::take(&mut *out).into_inner();
        String::from_utf8(out).ok()
    })
}

/// Provide a bunch of `std` functions which does something appropriate to the
/// wasm context.
pub fn wasm_module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.function(&["print"], print_impl)?;
    module.function(&["println"], println_impl)?;
    module.raw_fn(&["dbg"], dbg_impl)?;
    Ok(module)
}

fn print_impl(m: &str) -> Result<(), Panic> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();
        write!(out, "{}", m).map_err(Panic::custom)
    })
}

fn println_impl(m: &str) -> Result<(), Panic> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();
        writeln!(out, "{}", m).map_err(Panic::custom)
    })
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    use std::io::Write as _;

    OUT.with(|out| {
        let mut out = out.borrow_mut();

        for value in stack.drain_stack_top(args)? {
            writeln!(out, "{:?}", value).map_err(VmError::panic)?;
        }

        stack.push(Value::Unit);
        Ok(())
    })
}
