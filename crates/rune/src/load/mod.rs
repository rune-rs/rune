use crate::compiling;
use crate::{Diagnostics, Options};
use runestick::{Context, Unit};
use std::rc::Rc;
use thiserror::Error;

mod source_loader;
mod sources;

pub use self::source_loader::{FileSourceLoader, SourceLoader};
pub use self::sources::Sources;

/// Error raised when we failed to load sources.
///
/// Look at the passed in [Diagnostics] instance for details.
#[derive(Debug, Error)]
#[error("failed to load sources (see `errors` for details)")]
pub struct LoadSourcesError;

/// Load and compile the given sources.
///
/// Uses the [Source::name](runestick::Source::name) when generating diagnostics
/// to reference the file.
///
/// # Examples
///
/// Note: these must be built with the `diagnostics` feature enabled to give
/// access to `rune::termcolor`.
///
/// ```rust
/// use rune::termcolor::{ColorChoice, StandardStream};
/// use rune::EmitDiagnostics as _;
///
/// use std::path::Path;
/// use std::sync::Arc;
/// use std::error::Error;
///
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let context = runestick::Context::with_default_modules()?;
/// let mut options = rune::Options::default();
///
/// let mut sources = rune::Sources::new();
/// sources.insert(runestick::Source::new("entry", r#"
/// pub fn main() {
///     println("Hello World");
/// }
/// "#));
///
/// let mut diagnostics = rune::Diagnostics::new();
///
/// let result = rune::load_sources(&context, &options, &mut sources, &mut diagnostics);
///
/// if !diagnostics.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     diagnostics.emit_diagnostics(&mut writer, &sources)?;
/// }
///
/// let unit = result?;
/// let unit = Arc::new(unit);
/// let vm = runestick::Vm::new(Arc::new(context.runtime()), unit.clone());
/// # Ok(())
/// # }
/// ```
pub fn load_sources(
    context: &Context,
    options: &Options,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
) -> Result<Unit, LoadSourcesError> {
    let visitor = Rc::new(compiling::NoopCompileVisitor::new());
    let source_loader = Rc::new(FileSourceLoader::new());

    load_sources_with_visitor(
        context,
        options,
        sources,
        diagnostics,
        visitor,
        source_loader,
    )
}

/// Load the specified sources with a visitor.
pub fn load_sources_with_visitor<'a>(
    context: &Context,
    options: &Options,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
    visitor: Rc<dyn compiling::CompileVisitor>,
    source_loader: Rc<dyn SourceLoader + 'a>,
) -> Result<Unit, LoadSourcesError> {
    let unit = if context.has_default_modules() {
        compiling::UnitBuilder::with_default_prelude()
    } else {
        compiling::UnitBuilder::default()
    };

    let result = compiling::compile_with_options(
        &*context,
        sources,
        &unit,
        diagnostics,
        options,
        visitor,
        source_loader,
    );

    if let Err(()) = result {
        return Err(LoadSourcesError);
    }

    if options.link_checks {
        unit.link(&*context, diagnostics);

        if diagnostics.has_error() {
            return Err(LoadSourcesError);
        }
    }

    match unit.build() {
        Ok(unit) => Ok(unit),
        Err(error) => {
            diagnostics.error(0, error);
            Err(LoadSourcesError)
        }
    }
}
