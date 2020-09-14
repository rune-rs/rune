use crate::unit_builder::LinkerErrors;
use crate::unit_builder::UnitBuilder;
use crate::{compiler, CompileVisitor};
use crate::{
    FileSourceLoader, LoadError, LoadErrorKind, NoopCompileVisitor, Options, SourceLoader, Sources,
    Warnings,
};
use runestick::{Context, Source, Unit};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Load the given path.
///
/// The name of the loaded source will be the path as a string.
///
/// If you want to load a script from memory use [load_sources].
///
/// [load_sources]: crate::load_sources
///
/// # Examples
///
/// Note: these must be built with the `diagnostics` feature enabled to give
/// access to `rune::termcolor`.
///
/// ```rust,no_run
/// use rune::termcolor::{ColorChoice, StandardStream};
/// use rune::EmitDiagnostics as _;
///
/// use std::path::Path;
/// use std::sync::Arc;
/// use std::error::Error;
///
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let path = Path::new("script.rn");
///
/// let context = Arc::new(rune::default_context()?);
/// let mut options = rune::Options::default();
/// let mut sources = rune::Sources::new();
/// let mut warnings = rune::Warnings::new();
///
/// let unit = match rune::load_path(&*context, &options, &mut sources, &path, &mut warnings) {
///     Ok(unit) => unit,
///     Err(error) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         error.emit_diagnostics(&mut writer, &sources)?;
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
pub fn load_path(
    context: &Context,
    options: &Options,
    sources: &mut Sources,
    path: &Path,
    warnings: &mut Warnings,
) -> Result<Unit, LoadError> {
    sources.insert(Source::from_path(path).map_err(|error| {
        LoadError::from(LoadErrorKind::ReadFile {
            error,
            path: path.to_owned(),
        })
    })?);

    let unit = load_sources(context, options, sources, warnings)?;
    Ok(unit)
}

/// Load and compile the given source.
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
/// let mut warnings = rune::Warnings::new();
///
/// sources.insert(Source::new("entry", r#"
/// fn main() {
///     println("Hello World");
/// }
/// "#));
///
/// let unit = match rune::load_sources(&*context, &options, &mut sources, &mut warnings) {
///     Ok(unit) => unit,
///     Err(error) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         error.emit_diagnostics(&mut writer, &sources)?;
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
    warnings: &mut Warnings,
) -> Result<Unit, LoadError> {
    let mut visitor = NoopCompileVisitor::new();
    let mut source_loader = FileSourceLoader::new();
    load_sources_with_visitor(
        context,
        options,
        sources,
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
    warnings: &mut Warnings,
    visitor: &mut dyn CompileVisitor,
    source_loader: &mut dyn SourceLoader,
) -> Result<Unit, LoadError> {
    let unit = if context.has_default_modules() {
        UnitBuilder::with_default_prelude()
    } else {
        UnitBuilder::default()
    };

    let unit = Rc::new(RefCell::new(unit));
    compiler::compile_with_options(
        &*context,
        sources,
        &unit,
        warnings,
        &options,
        visitor,
        source_loader,
    )?;

    let unit = match Rc::try_unwrap(unit) {
        Ok(unit) => unit.into_inner(),
        Err(..) => {
            return Err(LoadError::from(LoadErrorKind::Internal {
                message: "unit is not exlusively held",
            }));
        }
    };

    if options.link_checks {
        let mut errors = LinkerErrors::new();

        if !unit.link(&*context, &mut errors) {
            return Err(LoadError::from(LoadErrorKind::LinkError { errors }));
        }
    }

    Ok(unit.into_unit())
}
