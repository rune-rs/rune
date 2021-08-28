//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/book/">
//!     <b>Read the book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! Very basic WASM bindings for Rune.
//!
//! This is part of the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io

use wasm_bindgen::prelude::*;

use anyhow::Context as _;
use rune::{DumpInstructions as _, EmitDiagnostics as _, Spanned as _};
use runestick::budget;
use runestick::{ContextError, Value};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

mod core;
mod http;
mod time;

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

#[derive(Deserialize)]
struct Config {
    /// Budget.
    #[serde(default)]
    budget: Option<usize>,
    /// Compiler options.
    #[serde(default)]
    options: Vec<String>,
    /// Include the `std::experiments` package.
    #[serde(default)]
    experimental: bool,
    /// Show instructions.
    #[serde(default)]
    instructions: bool,
    /// Suppress text warnings.
    #[serde(default)]
    suppress_text_warnings: bool,
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
    instructions: Option<String>,
}

impl CompileResult {
    /// Construct output from compile result.
    fn output(
        output: Value,
        diagnostics_output: Option<String>,
        diagnostics: Vec<Diagnostic>,
        instructions: Option<String>,
    ) -> Self {
        Self {
            error: None,
            diagnostics_output,
            diagnostics,
            result: Some(format!("{:?}", output)),
            output: core::drain_output(),
            instructions,
        }
    }

    /// Construct a result from an error.
    fn from_error<E>(
        error: E,
        diagnostics_output: Option<String>,
        diagnostics: Vec<Diagnostic>,
        instructions: Option<String>,
    ) -> Self
    where
        E: fmt::Display,
    {
        Self {
            error: Some(error.to_string()),
            diagnostics_output,
            diagnostics,
            result: None,
            output: core::drain_output(),
            instructions,
        }
    }
}

/// Setup a wasm-compatible context.
fn setup_context(experimental: bool) -> Result<runestick::Context, ContextError> {
    let mut context = runestick::Context::with_config(false)?;
    context.install(&core::module()?)?;
    context.install(&time::module()?)?;
    context.install(&http::module()?)?;
    context.install(&rune_modules::json::module(false)?)?;
    context.install(&rune_modules::toml::module(false)?)?;
    context.install(&rune_modules::rand::module(false)?)?;
    context.install(&rune_modules::core::module(false)?)?;
    context.install(&rune_modules::test::module(false)?)?;
    context.install(&rune_modules::io::module(false)?)?;
    context.install(&rune_modules::macros::module(false)?)?;

    if experimental {
        context.install(&rune_modules::experiments::module(false)?)?;
    }

    Ok(context)
}

async fn inner_compile(input: String, config: JsValue) -> Result<CompileResult, anyhow::Error> {
    let instructions = None;

    let config = config.into_serde::<Config>()?;
    let budget = config.budget.unwrap_or(1_000_000);

    let source = runestick::Source::new("entry", input);
    let mut sources = rune::Sources::new();
    sources.insert(source);

    let context = setup_context(config.experimental)?;

    let mut options = rune::Options::default();

    for option in &config.options {
        options.parse_option(option)?;
    }

    let mut d = rune::Diagnostics::new();
    let mut diagnostics = Vec::new();

    let result = rune::load_sources(&context, &options, &mut sources, &mut d);

    for diagnostic in d.diagnostics() {
        match diagnostic {
            rune::Diagnostic::Error(error) => {
                if let Some(source) = sources.get(error.source_id()) {
                    match error.kind() {
                        rune::ErrorKind::ParseError(error) => {
                            let span = error.span();

                            let start = Position::from(
                                source.position_to_unicode_line_char(span.start.into_usize()),
                            );
                            let end = Position::from(
                                source.position_to_unicode_line_char(span.end.into_usize()),
                            );

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: error.to_string(),
                            });
                        }
                        rune::ErrorKind::CompileError(error) => {
                            let span = error.span();

                            let start = Position::from(
                                source.position_to_unicode_line_char(span.start.into_usize()),
                            );
                            let end = Position::from(
                                source.position_to_unicode_line_char(span.end.into_usize()),
                            );

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: error.to_string(),
                            });
                        }
                        rune::ErrorKind::QueryError(error) => {
                            let span = error.span();

                            let start = Position::from(
                                source.position_to_unicode_line_char(span.start.into_usize()),
                            );
                            let end = Position::from(
                                source.position_to_unicode_line_char(span.end.into_usize()),
                            );

                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::Error,
                                start,
                                end,
                                message: error.to_string(),
                            });
                        }
                        rune::ErrorKind::LinkError(error) => match error {
                            rune::LinkerError::MissingFunction { hash, spans } => {
                                for (span, _) in spans {
                                    let start = Position::from(
                                        source
                                            .position_to_unicode_line_char(span.start.into_usize()),
                                    );
                                    let end = Position::from(
                                        source.position_to_unicode_line_char(span.end.into_usize()),
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
                        rune::ErrorKind::Internal(_) => {}
                        rune::ErrorKind::BuildError(_) => {}
                    }
                }
            }
            rune::Diagnostic::Warning(warning) => {
                let span = warning.span();

                if let Some(source) = sources.get(warning.source_id()) {
                    let start = Position::from(
                        source.position_to_unicode_line_char(span.start.into_usize()),
                    );
                    let end =
                        Position::from(source.position_to_unicode_line_char(span.end.into_usize()));

                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Warning,
                        start,
                        end,
                        message: warning.to_string(),
                    });
                }
            }
        }
    }

    let mut writer = rune::termcolor::Buffer::no_color();

    if !config.suppress_text_warnings {
        d.emit_diagnostics(&mut writer, &sources)
            .context("emitting to buffer should never fail")?;
    }

    let unit = match result {
        Ok(unit) => Arc::new(unit),
        Err(error) => {
            return Ok(CompileResult::from_error(
                error,
                diagnostics_output(writer),
                diagnostics,
                instructions,
            ));
        }
    };

    let instructions = if config.instructions {
        let mut out = rune::termcolor::Buffer::no_color();
        unit.dump_instructions(&mut out, &sources, false)
            .expect("dumping to string shouldn't fail");
        Some(diagnostics_output(out).context("converting instructions to UTF-8")?)
    } else {
        None
    };

    let vm = runestick::Vm::new(Arc::new(context.runtime()), unit);

    let mut execution = match vm.execute(&["main"], ()) {
        Ok(execution) => execution,
        Err(error) => {
            error
                .emit_diagnostics(&mut writer, &sources)
                .context("emitting to buffer should never fail")?;

            return Ok(CompileResult::from_error(
                error,
                diagnostics_output(writer),
                diagnostics,
                instructions,
            ));
        }
    };

    let future = budget::with(budget, execution.async_complete());

    let output = match future.await {
        Ok(output) => output,
        Err(error) => {
            if let Ok(vm) = execution.vm() {
                let (kind, unwound) = error.as_unwound();

                let (unit, ip, _frames) = match unwound {
                    Some((unit, ip, frames)) => (unit, ip, frames),
                    None => (vm.unit(), vm.ip(), vm.call_frames().to_owned()),
                };

                // NB: emit diagnostics if debug info is available.
                if let Some(debug) = unit.debug_info() {
                    if let Some(inst) = debug.instruction_at(ip) {
                        if let Some(source) = sources.get(inst.source_id) {
                            let start = Position::from(
                                source.position_to_unicode_line_char(inst.span.start.into_usize()),
                            );
                            let end = Position::from(
                                source.position_to_unicode_line_char(inst.span.end.into_usize()),
                            );

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
                .context("emitting to buffer should never fail")?;

            return Ok(CompileResult::from_error(
                error,
                diagnostics_output(writer),
                diagnostics,
                instructions,
            ));
        }
    };

    Ok(CompileResult::output(
        output,
        diagnostics_output(writer),
        diagnostics,
        instructions,
    ))
}

fn diagnostics_output(writer: rune::termcolor::Buffer) -> Option<String> {
    let mut string = String::from_utf8(writer.into_inner()).ok()?;
    let new_len = string.trim_end().len();
    string.truncate(new_len);
    Some(string)
}

#[wasm_bindgen]
pub async fn compile(input: String, config: JsValue) -> JsValue {
    let result = match inner_compile(input, config).await {
        Ok(result) => result,
        Err(error) => CompileResult::from_error(error, None, Vec::new(), None),
    };

    JsValue::from_serde(&result).unwrap()
}
