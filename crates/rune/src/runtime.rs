use crate::error::{CompileError, ConfigurationError, ParseError};
use crate::options::Options;
use crate::warning::{Warning, Warnings};
use runestick::unit::{LinkerError, LinkerErrors, Span};
use slab::Slab;
use std::error::Error as _;
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::{Files, SimpleFile};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

pub use codespan_reporting::term::termcolor;

/// An error that occurs during a load.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Failed to read the given file.
    #[error("failed to read file `{path}`")]
    ReadFile {
        /// The source error.
        #[source]
        error: io::Error,
        /// The path that failed to load.
        path: PathBuf,
    },
    /// When we try to read a file that is missing.
    #[error("tried to read a missing file")]
    MissingFile,
    /// A parse error.
    #[error("failed to parse the file")]
    ParseError,
    /// A compile error.
    #[error("failed to compile the file")]
    CompileError,
    /// A linker error.
    #[error("failed to link the loaded file")]
    LinkError,
}

/// A runtime error.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Virtual machine errors.
    #[error("virtual machine error")]
    VmError {
        /// The source error.
        #[source]
        error: runestick::VmError,
        /// The span at which the error occured.
        span: Span,
    },
    /// Source parse error.
    #[error("parse error")]
    ParseError {
        #[from]
        error: ParseError,
    },
    /// Compiler error.
    #[error("compile error")]
    CompileError {
        #[from]
        error: CompileError,
    },
    /// A linker error occured.
    #[error("linker error")]
    LinkError {
        /// Errors that happened during linking.
        errors: LinkerErrors,
    },
}

/// Error emitted when we saw an error while we were emitting diagnostics.
#[derive(Debug, Error)]
pub enum DiagnosticsError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("formatting error")]
    Fmt(#[from] fmt::Error),
}

/// A rune runtime, which simplifies embedding and using rune.
pub struct Runtime {
    context: Rc<runestick::Context>,
    files: SlabFiles,
    options: Options,
    errors: Vec<(usize, RuntimeError)>,
    warnings: Vec<(usize, Warnings)>,
    linking: bool,
}

impl Runtime {
    /// Construct a new runtime with the default context.
    pub fn new() -> Result<Self, runestick::ContextError> {
        Ok(Self::with_context(
            runestick::Context::with_default_packages()?,
        ))
    }

    /// Construct a runtime with a default context.
    ///
    /// If built with the `modules` feature, this includes all available native
    /// modules.
    pub fn with_default_context() -> Result<Self, runestick::ContextError> {
        #[allow(unused_mut)]
        let mut context = runestick::Context::with_default_packages()?;

        #[cfg(feature = "modules")]
        {
            context.install(&rune_modules::http::module()?)?;
            context.install(&rune_modules::json::module()?)?;
            context.install(&rune_modules::time::module()?)?;
            context.install(&rune_modules::process::module()?)?;
        }

        Ok(Self::with_context(context))
    }

    /// Construct a new runtime with a custom context.
    pub fn with_context(context: runestick::Context) -> Self {
        Self {
            context: Rc::new(context),
            files: SlabFiles::new(),
            options: crate::Options::default(),
            errors: Vec::new(),
            warnings: Default::default(),
            linking: true,
        }
    }

    /// Disable linking, which will otherwise check that all function signatures
    /// exist for all function calls at compile time.
    pub fn disable_linking(&mut self) {
        self.linking = false;
    }

    /// Indicate that the runtime has issues it can report with
    /// [emit_diagnostics][Self::emit_diagnostics].
    pub fn has_issues(&self) -> bool {
        self.errors.is_empty() && !self.warnings.is_empty()
    }

    /// Access the underlying context of the runtime.
    pub fn context(&self) -> &runestick::Context {
        &self.context
    }

    /// Get the unit associated with the given file id.
    pub fn unit_vm(&self, file_id: usize) -> Option<runestick::Vm> {
        let unit = self.files.get(file_id)?.unit.as_ref()?.clone();
        let vm = runestick::Vm::new(self.context.clone(), unit);
        Some(vm)
    }

    /// Register the runtime error.
    ///
    /// If we don't have debuginfo, returns Err with the passed in error.
    pub fn register_vm_error(
        &mut self,
        top_ip: usize,
        file_id: usize,
        error: runestick::VmError,
    ) -> Result<(), runestick::VmError> {
        let unit = match self.files.get(file_id).and_then(|f| f.unit.as_ref()) {
            Some(unit) => unit,
            None => return Err(error),
        };

        let (error, ip) = error.from_unwinded();
        let ip = ip.unwrap_or(top_ip);

        if let Some(debug) = unit.debug_info_at(ip) {
            self.errors.push((
                file_id,
                RuntimeError::VmError {
                    error,
                    span: debug.span,
                },
            ));

            Ok(())
        } else {
            Err(error)
        }
    }

    /// Prase the given optimization options.
    pub fn parse_optimization<I>(&mut self, options: I) -> Result<(), ConfigurationError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for opt in options {
            self.options.parse_option(opt.as_ref())?;
        }

        Ok(())
    }

    /// Load the given path into the runtime.
    pub fn load(&mut self, path: &Path) -> Result<usize, LoadError> {
        let source = fs::read_to_string(path).map_err(|error| LoadError::ReadFile {
            error,
            path: path.to_owned(),
        })?;

        let file_id = self.files.add(path.display().to_string(), source);

        let file = match self.files.borrow_mut(file_id) {
            Some(file) => file,
            None => {
                return Err(LoadError::MissingFile);
            }
        };

        let unit = match crate::parse_all::<crate::ast::DeclFile>(file.file.source()) {
            Ok(unit) => unit,
            Err(e) => {
                self.errors.push((file_id, e.into()));
                return Err(LoadError::ParseError);
            }
        };

        let (unit, warnings) = match unit.compile_with_options(&self.context, &self.options) {
            Ok(unit) => unit,
            Err(e) => {
                self.errors.push((file_id, e.into()));
                return Err(LoadError::CompileError);
            }
        };

        if !warnings.is_empty() {
            self.warnings.push((file_id, warnings));
        }

        let mut errors = LinkerErrors::new();

        if self.linking {
            if !unit.link(&self.context, &mut errors) {
                file.unit = Some(Rc::new(unit));
                self.errors
                    .push((file_id, RuntimeError::LinkError { errors }));
                return Err(LoadError::LinkError);
            }
        }

        file.unit = Some(Rc::new(unit));
        Ok(file_id)
    }

    /// Emit diagnostics about the last error we encountered.
    pub fn emit_diagnostics<O>(&mut self, out: &mut O) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let errors = std::mem::take(&mut self.errors);
        let config = codespan_reporting::term::Config::default();

        for (source_file, error) in errors {
            let mut labels = Vec::new();

            let span = match &error {
                RuntimeError::LinkError { errors } => {
                    for error in errors {
                        match error {
                            LinkerError::MissingFunction { hash, spans } => {
                                let mut labels = Vec::new();

                                for span in spans {
                                    labels.push(
                                        Label::primary(source_file, span.start..span.end)
                                            .with_message("called here."),
                                    );
                                }

                                let diagnostic = Diagnostic::error()
                                    .with_message(format!("missing function with hash `{}`", hash))
                                    .with_labels(labels);

                                term::emit(out, &config, &self.files, &diagnostic)?;
                            }
                        }
                    }

                    return Ok(());
                }
                RuntimeError::VmError { span, .. } => *span,
                RuntimeError::CompileError { error } => match error {
                    CompileError::ReturnLocalReferences {
                        block,
                        references_at,
                        span,
                        ..
                    } => {
                        for ref_span in references_at {
                            if span.overlaps(*ref_span) {
                                continue;
                            }

                            labels.push(
                                Label::secondary(source_file, ref_span.start..ref_span.end)
                                    .with_message("reference created here"),
                            );
                        }

                        labels.push(
                            Label::secondary(source_file, block.start..block.end)
                                .with_message("block returned from"),
                        );

                        *span
                    }
                    CompileError::DuplicateObjectKey {
                        span,
                        existing,
                        object,
                    } => {
                        labels.push(
                            Label::secondary(source_file, existing.start..existing.end)
                                .with_message("previously defined here"),
                        );

                        labels.push(
                            Label::secondary(source_file, object.start..object.end)
                                .with_message("object being defined here"),
                        );

                        *span
                    }
                    error => error.span(),
                },
                RuntimeError::ParseError { error } => error.span(),
            };

            if let Some(e) = error.source() {
                labels.push(
                    Label::primary(source_file, span.start..span.end).with_message(e.to_string()),
                );
            }

            let diagnostic = Diagnostic::error()
                .with_message(error.to_string())
                .with_labels(labels);

            term::emit(out, &config, &self.files, &diagnostic)?;
        }

        let warnings = std::mem::take(&mut self.warnings);

        for (source_file, warnings) in warnings {
            let mut labels = Vec::new();
            let mut notes = Vec::new();

            for warning in warnings {
                let context = match warning {
                    Warning::NotUsed { span, context } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("value not used"),
                        );

                        context
                    }
                    Warning::LetPatternMightPanic { span, context } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("let binding might panic"),
                        );

                        if let Some(binding) = self
                            .files
                            .source(source_file)
                            .and_then(|s| s.get(span.start..span.end))
                        {
                            let mut note = String::new();
                            writeln!(note, "Consider rewriting to:")?;
                            writeln!(note, "if {} {{", binding)?;
                            writeln!(note, "    // ..")?;
                            writeln!(note, "}}")?;
                            notes.push(note);
                        }

                        context
                    }
                    Warning::BreakDoesNotProduceValue { span, context } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("break expressions do not produce a value"),
                        );
                        context
                    }
                    Warning::TemplateWithoutExpansions { span, context } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("template string without expansions like `{1 + 2}`"),
                        );

                        context
                    }
                    Warning::RemoveTupleCallParams {
                        span,
                        variant,
                        context,
                    } => {
                        labels.push(
                            Label::secondary(source_file, span.start..span.end).with_message(
                                "constructing this variant could be done without parentheses",
                            ),
                        );

                        if let Some(variant) = self
                            .files
                            .source(source_file)
                            .and_then(|s| s.get(variant.start..variant.end))
                        {
                            let mut note = String::new();
                            writeln!(note, "Consider rewriting to `{}`", variant)?;
                            notes.push(note);
                        }

                        context
                    }
                    Warning::UnecessarySemiColon { span } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("unnecessary semicolon"),
                        );

                        None
                    }
                };

                if let Some(context) = context {
                    labels.push(
                        Label::secondary(source_file, context.start..context.end)
                            .with_message("in this context"),
                    );
                }
            }

            let diagnostic = Diagnostic::warning()
                .with_message("warning")
                .with_labels(labels)
                .with_notes(notes);

            term::emit(out, &config, &self.files, &diagnostic)?;
        }

        Ok(())
    }
}

struct File {
    file: SimpleFile<String, String>,
    unit: Option<Rc<runestick::Unit>>,
}

struct SlabFiles {
    files: Slab<File>,
}

impl SlabFiles {
    fn new() -> Self {
        Self { files: Slab::new() }
    }

    fn add(&mut self, name: String, source: String) -> usize {
        self.files.insert(File {
            file: SimpleFile::new(name, source),
            unit: None,
        })
    }

    fn borrow_mut(&mut self, file_id: usize) -> Option<&mut File> {
        self.files.get_mut(file_id)
    }

    fn get(&self, file_id: usize) -> Option<&File> {
        self.files.get(file_id)
    }
}

impl<'a> Files<'a> for SlabFiles {
    type FileId = usize;
    type Name = String;
    type Source = &'a str;

    fn name(&self, file_id: usize) -> Option<String> {
        Some(self.get(file_id)?.file.name().clone())
    }

    fn source(&self, file_id: usize) -> Option<&str> {
        Some(self.get(file_id)?.file.source().as_ref())
    }

    fn line_index(&self, file_id: usize, byte_index: usize) -> Option<usize> {
        self.get(file_id)?.file.line_index((), byte_index)
    }

    fn line_range(&self, file_id: usize, line_index: usize) -> Option<Range<usize>> {
        self.get(file_id)?.file.line_range((), line_index)
    }
}
