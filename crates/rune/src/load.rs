use crate::unit_builder::UnitBuilder;
use crate::{compiler, CompileVisitor};
use crate::{
    Errors, FileSourceLoader, LoadError, NoopCompileVisitor, Options, SourceLoader, Sources,
    Warnings,
};
use runestick::{Context, Unit};
use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;

/// Error raised when we failed to load sources.
///
/// Look at the passed in [Errors] instance for details.
#[derive(Debug, Error)]
#[error("failed to load sources (see `errors` for details)")]
pub struct LoadSourcesError;

/// Load and compile the given sources.
///
/// Uses the [Source::name] when generating diagnostics to reference the file.
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
/// let context = Arc::new(rune::default_context()?);
/// let mut options = rune::Options::default();
/// let mut sources = rune::Sources::new();
/// sources.insert(Source::new("entry", r#"
/// fn main() {
///     println("Hello World");
/// }
/// "#));
///
/// let mut errors = rune::Errors::new();
/// let mut warnings = rune::Warnings::new();
///
/// let unit = match rune::load_sources(&*context, &options, &mut sources, &mut errors, &mut warnings) {
///     Ok(unit) => unit,
///     Err(rune::LoadSourcesError) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         errors.emit_diagnostics(&mut writer, &sources)?;
///         return Ok(());
///     }
/// };
///
/// let unit = Arc::new(unit);
/// let vm = runestick::Vm::new(context.clone(), unit.clone());
///
/// if !warnings.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     warnings.emit_diagnostics(&mut writer, &sources)?;
/// }
///
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
    let mut visitor = NoopCompileVisitor::new();
    let mut source_loader = FileSourceLoader::new();

    load_sources_with_visitor(
        context,
        options,
        sources,
        errors,
        warnings,
        &mut visitor,
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
    visitor: &mut dyn CompileVisitor,
    source_loader: &mut dyn SourceLoader,
) -> Result<Unit, LoadSourcesError> {
    let unit = if context.has_default_modules() {
        UnitBuilder::with_default_prelude()
    } else {
        UnitBuilder::default()
    };

    let unit = Rc::new(RefCell::new(unit));

    let result = compiler::compile_with_options(
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

    let unit = match Rc::try_unwrap(unit) {
        Ok(unit) => unit.into_inner(),
        Err(..) => {
            errors.push(LoadError::internal(0, "unit is not exlusively held"));

            return Err(LoadSourcesError);
        }
    };

    if options.link_checks {
        unit.link(&*context, errors);

        if !errors.is_empty() {
            return Err(LoadSourcesError);
        }
    }

    Ok(unit.into_unit())
}
