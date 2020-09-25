//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
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

use rune::{EmitDiagnostics as _, Spanned as _};
use runestick::budget;
use runestick::{ContextError, Value};
use serde::Serialize;
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
            output: core::drain_output(),
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
            output: core::drain_output(),
        }
    }
}

/// Setup a wasm-compatible context.
fn setup_context() -> Result<runestick::Context, ContextError> {
    let mut context = runestick::Context::with_config(false)?;
    context.install(&core::module()?)?;
    context.install(&time::module()?)?;
    context.install(&http::module()?)?;
    context.install(&rune_modules::json::module()?)?;
    context.install(&rune_modules::toml::module()?)?;
    context.install(&rune_modules::rand::module()?)?;
    Ok(context)
}

async fn inner_compile(input: String, budget: usize) -> CompileResult {
    let source = runestick::Source::new("entry", input);
    let mut sources = rune::Sources::new();
    sources.insert(source);

    let context = match setup_context() {
        Ok(context) => context,
        Err(error) => {
            return CompileResult::from_error(error, None, Vec::new());
        }
    };

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
                        rune::LoadErrorKind::QueryError(error) => {
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

    let output = match future.await {
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
pub async fn compile(input: String, budget: usize) -> JsValue {
    JsValue::from_serde(&inner_compile(input, budget).await).unwrap()
}
