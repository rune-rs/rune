use crate::compiling;
use crate::Options;
use runestick::{Context, Unit};
use std::rc::Rc;
use thiserror::Error;

mod error;
mod errors;
mod source_loader;
mod sources;
mod warning;
mod warnings;

pub use self::error::{Error, ErrorKind};
pub use self::errors::Errors;
pub use self::source_loader::{FileSourceLoader, SourceLoader};
pub use self::sources::Sources;
pub use self::warning::{Warning, WarningKind};
pub use self::warnings::Warnings;

/// Error raised when we failed to load sources.
///
/// Look at the passed in [Errors] instance for details.
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
/// use runestick::Source;
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
/// sources.insert(Source::new("entry", r#"
/// pub fn main() {
///     println("Hello World");
/// }
/// "#));
///
/// let mut errors = rune::Errors::new();
/// let mut warnings = rune::Warnings::new();
///
/// let unit = match rune::load_sources(&context, &options, &mut sources, &mut errors, &mut warnings) {
///     Ok(unit) => unit,
///     Err(rune::LoadSourcesError) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         errors.emit_diagnostics(&mut writer, &sources)?;
///         return Ok(());
///     }
/// };
///
/// let unit = Arc::new(unit);
/// let vm = runestick::Vm::new(Arc::new(context.runtime()), unit.clone());
///
/// if !warnings.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     warnings.emit_diagnostics(&mut writer, &sources)?;
/// }
/// # Ok(())
/// # }
/// ```
pub fn load_sources(
    context: &Context,
    options: &Options,
    sources: &mut Sources,
    errors: &mut Errors,
    warnings: &mut Warnings,
) -> Result<Unit, LoadSourcesError> {
    let visitor = Rc::new(compiling::NoopCompileVisitor::new());
    let mut source_loader = FileSourceLoader::new();

    load_sources_with_visitor(
        context,
        options,
        sources,
        errors,
        warnings,
        visitor,
        &mut source_loader,
    )
}

/// Load the specified sources with a visitor.
pub fn load_sources_with_visitor(
    context: &Context,
    options: &Options,
    sources: &mut Sources,
    errors: &mut Errors,
    warnings: &mut Warnings,
    visitor: Rc<dyn compiling::CompileVisitor>,
    source_loader: &mut dyn SourceLoader,
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
        errors,
        warnings,
        &options,
        visitor,
        source_loader,
    );

    if let Err(()) = result {
        return Err(LoadSourcesError);
    }

    if options.link_checks {
        unit.link(&*context, errors);

        if !errors.is_empty() {
            return Err(LoadSourcesError);
        }
    }

    match unit.build() {
        Ok(unit) => Ok(unit),
        Err(error) => {
            errors.push(Error::new(0, error));
            Err(LoadSourcesError)
        }
    }
}
