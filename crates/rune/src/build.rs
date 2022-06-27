use crate::ast::Span;
use crate::compile;
use crate::compile::{CompileVisitor, FileSourceLoader, NoopCompileVisitor, Options, SourceLoader};
use crate::runtime::Unit;
use crate::{Context, Diagnostics, SourceId, Sources};
use thiserror::Error;

/// Error raised when we failed to load sources.
///
/// Look at the passed in [Diagnostics] instance for details.
#[derive(Debug, Error)]
#[error("failed to build rune sources (see diagnostics for details)")]
#[non_exhaustive]
pub struct BuildError;

/// Entry point to building [Sources] of Rune.
///
/// Uses the [Source::name](crate::Source::name) when generating diagnostics to
/// reference the file.
///
/// # Examples
///
/// Note: these must be built with the `emit` feature enabled (default) to give
/// access to `rune::termcolor`.
///
/// ```
/// use rune::termcolor::{ColorChoice, StandardStream};
/// use rune::{Context, Options, Source, Vm};
/// use std::sync::Arc;
///
/// # fn main() -> rune::Result<()> {
/// let context = Context::with_default_modules()?;
/// let runtime = Arc::new(context.runtime());
///
/// let mut sources = rune::Sources::new();
/// sources.insert(Source::new("entry", r#"
/// pub fn main() {
///     println("Hello World");
/// }
/// "#));
///
/// let mut diagnostics = rune::Diagnostics::new();
///
/// let result = rune::prepare(&mut sources)
///     .with_context(&context)
///     .with_diagnostics(&mut diagnostics)
///     .build();
///
/// if !diagnostics.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     diagnostics.emit(&mut writer, &sources)?;
/// }
///
/// let unit = result?;
/// let unit = Arc::new(unit);
/// let vm = Vm::new(runtime, unit);
/// # Ok(()) }
/// ```
pub fn prepare(sources: &mut Sources) -> Build<'_> {
    Build {
        sources,
        context: None,
        diagnostics: None,
        options: None,
        visitor: None,
        source_loader: None,
    }
}

/// High level helper for setting up a build of Rune sources into a [Unit].
pub struct Build<'a> {
    sources: &'a mut Sources,
    context: Option<&'a Context>,
    diagnostics: Option<&'a mut Diagnostics>,
    options: Option<&'a Options>,
    visitor: Option<&'a mut dyn compile::CompileVisitor>,
    source_loader: Option<&'a mut dyn SourceLoader>,
}

impl<'a> Build<'a> {
    /// Modify the current [Build] to use the given [Context] while building.
    ///
    /// If unspecified the empty context constructed with [Context::new] will be
    /// used. Since this counts as building without a context,
    /// [Vm::without_context][crate::runtime::Vm] can be used when running the
    /// produced [Unit].
    #[inline]
    pub fn with_context(mut self, context: &'a Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Modify the current [Build] to use the given [Diagnostics] collection.
    #[inline]
    pub fn with_diagnostics(mut self, diagnostics: &'a mut Diagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// Modify the current [Build] to use the given [Options].
    #[inline]
    pub fn with_options(mut self, options: &'a Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Modify the current [Build] to configure the given [CompileVisitor].
    ///
    /// A compile visitor allows for custom collecting of compile-time metadata.
    /// Like if you want to collect every function that is discovered in the
    /// project.
    #[inline]
    pub fn with_visitor(mut self, visitor: &'a mut dyn CompileVisitor) -> Self {
        self.visitor = Some(visitor);
        self
    }

    /// Modify the current [Build] to configure the given [SourceLoader].
    ///
    /// Source loaders are used to determine how sources are loaded externally
    /// from the current file (as is neede when a module is imported).
    #[inline]
    pub fn with_source_loader(mut self, source_loader: &'a mut dyn SourceLoader) -> Self {
        self.source_loader = Some(source_loader);
        self
    }

    /// Build a [Unit] with the current configuration.
    pub fn build(mut self) -> Result<Unit, BuildError> {
        let default_context;

        let context = match self.context.take() {
            Some(context) => context,
            None => {
                default_context = Context::new();
                &default_context
            }
        };

        let mut unit = compile::UnitBuilder::default();

        let prelude = if context.has_default_modules() {
            compile::Prelude::with_default_prelude()
        } else {
            compile::Prelude::default()
        };

        let mut default_diagnostics;

        let diagnostics = match self.diagnostics.take() {
            Some(diagnostics) => diagnostics,
            None => {
                default_diagnostics = Diagnostics::new();
                &mut default_diagnostics
            }
        };

        let default_options;

        let options = match self.options.take() {
            Some(options) => options,
            None => {
                default_options = Options::default();
                &default_options
            }
        };

        let mut default_visitor;

        let visitor = match self.visitor.take() {
            Some(visitor) => visitor,
            None => {
                default_visitor = NoopCompileVisitor::new();
                &mut default_visitor
            }
        };

        let mut default_source_loader;

        let source_loader = match self.source_loader.take() {
            Some(source_loader) => source_loader,
            None => {
                default_source_loader = FileSourceLoader::new();
                &mut default_source_loader
            }
        };

        let result = compile::compile(
            &mut unit,
            &prelude,
            self.sources,
            context,
            diagnostics,
            options,
            visitor,
            source_loader,
        );

        if let Err(()) = result {
            return Err(BuildError);
        }

        if options.link_checks {
            unit.link(context, diagnostics);

            if diagnostics.has_error() {
                return Err(BuildError);
            }
        }

        match unit.build(Span::empty()) {
            Ok(unit) => Ok(unit),
            Err(error) => {
                diagnostics.error(SourceId::empty(), error);
                Err(BuildError)
            }
        }
    }
}
