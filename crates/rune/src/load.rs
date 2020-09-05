use crate::compiler;
use crate::{CompileError, LoadError, LoadErrorKind, Options, Warnings};
use runestick::{Context, LinkerErrors, Source, Span, Unit};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Load the given path.
///
/// The name of the loaded source will be the path as a string.
///
/// If you want to load a script from memory use [load_source].
///
/// [load_source]: crate::load_source
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
/// let mut warnings = rune::Warnings::new();
///
/// let unit = match rune::load_path(&*context, &options, &path, &mut warnings) {
///     Ok(unit) => unit,
///     Err(error) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         error.emit_diagnostics(&mut writer)?;
///         return Ok(());
///     }
/// };
///
/// let unit = Arc::new(unit);
/// let vm = runestick::Vm::new(context.clone(), unit.clone());
///
/// if !warnings.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     rune::emit_warning_diagnostics(&mut writer, &warnings, &*unit)?;
/// }
///
/// # Ok(())
/// # }
/// ```
pub fn load_path(
    context: &Context,
    options: &Options,
    path: &Path,
    warnings: &mut Warnings,
) -> Result<Unit, LoadError> {
    let source = fs::read_to_string(path).map_err(|error| {
        LoadError::from(LoadErrorKind::ReadFile {
            error,
            path: path.to_owned(),
        })
    })?;

    let name = path.display().to_string();
    let unit = load_source(context, options, Source::new(name, source), warnings)?;
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
/// let mut warnings = rune::Warnings::new();
///
/// let source = Source::new("entry", r#"
/// fn main() {
///     println("Hello World");
/// }
/// "#);
///
/// let unit = match rune::load_source(&*context, &options, source, &mut warnings) {
///     Ok(unit) => unit,
///     Err(error) => {
///         let mut writer = StandardStream::stderr(ColorChoice::Always);
///         error.emit_diagnostics(&mut writer)?;
///         return Ok(());
///     }
/// };
///
/// let unit = Arc::new(unit);
/// let vm = runestick::Vm::new(context.clone(), unit.clone());
///
/// if !warnings.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     rune::emit_warning_diagnostics(&mut writer, &warnings, &*unit)?;
/// }
///
/// # Ok(())
/// # }
/// ```
pub fn load_source(
    context: &Context,
    options: &Options,
    code_source: Source,
    warnings: &mut Warnings,
) -> Result<Unit, LoadError> {
    let unit = Rc::new(RefCell::new(Unit::with_default_prelude()));

    if let Err(error) =
        compiler::compile_with_options(&*context, &code_source, &options, &unit, warnings)
    {
        return Err(LoadError::from(LoadErrorKind::CompileError {
            error,
            code_source,
        }));
    }

    let unit = match Rc::try_unwrap(unit) {
        Ok(unit) => unit.into_inner(),
        Err(..) => {
            return Err(LoadError::from(LoadErrorKind::CompileError {
                error: CompileError::internal("unit is not exlusively held", Span::empty()),
                code_source,
            }));
        }
    };

    if options.link_checks {
        let mut errors = LinkerErrors::new();

        if !unit.link(&*context, &mut errors) {
            return Err(LoadError::from(LoadErrorKind::LinkError {
                errors,
                code_source,
            }));
        }
    }

    Ok(unit)
}
