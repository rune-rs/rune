use crate::compiler::{Options, Warning, Warnings};
use crate::error::{CompileError, ConfigurationError, ParseError, SpannedError as _};
use anyhow::Result;
use slab::Slab;
use st::unit::{LinkerError, LinkerErrors, Span};
use std::error::Error as _;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
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

/// An error that occurs when trying to call a function.
#[derive(Debug, Error)]
pub enum CallFunctionError {
    /// Vm error raised when trying to initiate a function call.
    #[error("error in virtual machine")]
    VmError {
        /// The error.
        #[from]
        error: st::VmError,
    },
    /// Error raised when we try to call a function on a missing unit.
    #[error("missing unit for file id `{file_id}`")]
    MissingUnit {
        /// The source id of the unit that was missing.
        file_id: usize,
    },
}

/// A runtime error.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Virtual machine errors.
    #[error("virtual machine error")]
    VmError {
        /// The source error.
        #[source]
        error: st::VmError,
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

/// A rune runtime, which simplifies embedding and using rune.
pub struct Runtime {
    /// The underlying virtual machine.
    vm: st::Vm,
    context: st::Context,
    files: SlabFiles,
    options: Options,
    errors: Vec<(usize, RuntimeError)>,
    warnings: Vec<(usize, Warnings)>,
}

impl Runtime {
    /// Construct a new runtime with the default context.
    pub fn new() -> Result<Self, st::ContextError> {
        Ok(Self::with_context(st::Context::with_default_packages()?))
    }

    /// Indicate that the runtime has issues it can report with
    /// [emit_diagnostics][Self::emit_diagnostics].
    pub fn has_issues(&self) -> bool {
        self.errors.is_empty() && !self.warnings.is_empty()
    }

    /// Construct a new runtime with a custom context.
    pub fn with_context(context: st::Context) -> Self {
        Self {
            context,
            vm: st::Vm::new(),
            files: SlabFiles::new(),
            options: crate::Options::default(),
            errors: Vec::new(),
            warnings: Default::default(),
        }
    }

    /// Access the underlying virtual machine of the runtime.
    pub fn vm(&self) -> &st::Vm {
        &self.vm
    }

    /// Access the underlying context of the runtime.
    pub fn context(&self) -> &st::Context {
        &self.context
    }

    /// Get the unit associated with the given file id.
    pub fn unit(&self, file_id: usize) -> Option<&st::CompilationUnit> {
        self.files.get(file_id)?.unit.as_ref()
    }

    /// Call the given function in the given named file.
    ///
    /// Returns the associated task and the file id associated with the unit.
    pub fn call_function<'a, A, T, I>(
        &'a mut self,
        file_id: usize,
        name: I,
        args: A,
    ) -> Result<st::Task<'a, T>, CallFunctionError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        A: st::IntoArgs,
        T: st::FromValue,
    {
        let unit = self
            .files
            .get(file_id)
            .and_then(|file| file.unit.as_ref())
            .ok_or_else(|| CallFunctionError::MissingUnit { file_id })?;

        Ok(self.vm.call_function(&self.context, unit, name, args)?)
    }

    /// Register the runtime error.
    ///
    /// If we don't have debuginfo, returns Err with the passed in error.
    pub fn register_vm_error(
        &mut self,
        file_id: usize,
        error: st::VmError,
    ) -> Result<(), st::VmError> {
        let unit = match self.files.get(file_id).and_then(|f| f.unit.as_ref()) {
            Some(unit) => unit,
            None => return Err(error),
        };

        if let Some(debug) = unit.debug_info_at(self.vm.ip()) {
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

        let file = match self.files.get_mut(file_id) {
            Some(file) => file,
            None => {
                return Err(LoadError::MissingFile);
            }
        };

        let unit = match crate::parse_all::<crate::ast::File>(file.file.source()) {
            Ok(unit) => unit,
            Err(e) => {
                self.errors.push((file_id, e.into()));
                return Err(LoadError::ParseError);
            }
        };

        let (unit, warnings) = match unit.compile_with_options(&self.options) {
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

        if !unit.link(&self.context, &mut errors) {
            file.unit = Some(unit);
            self.errors
                .push((file_id, RuntimeError::LinkError { errors }));
            return Err(LoadError::LinkError);
        }

        file.unit = Some(unit);
        Ok(file_id)
    }

    /// Emit diagnostics about the last error we encountered.
    pub fn emit_diagnostics<O>(&mut self, out: &mut O) -> Result<()>
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
                    CompileError::ReturnDoesNotProduceValue { block, span, .. } => {
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
                match warning {
                    Warning::NotUsed { span, context } => {
                        labels.push(
                            Label::primary(source_file, span.start..span.end)
                                .with_message("value not used"),
                        );

                        if let Some(context) = context {
                            labels.push(
                                Label::secondary(source_file, context.start..context.end)
                                    .with_message("in this context"),
                            );
                        }
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
                            writeln!(note, "Consider rewriting it to:")?;
                            writeln!(note, "if {} {{", binding)?;
                            writeln!(note, "    // ..")?;
                            writeln!(note, "}}")?;
                            notes.push(note);
                        }

                        if let Some(context) = context {
                            labels.push(
                                Label::secondary(source_file, context.start..context.end)
                                    .with_message("in this context"),
                            );
                        }
                    }
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
    unit: Option<st::CompilationUnit>,
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

    fn get_mut(&mut self, file_id: usize) -> Option<&mut File> {
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
