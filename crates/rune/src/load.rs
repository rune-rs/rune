use crate::compiler;
use crate::{CompileError, LoadError, LoadErrorKind, Options, Warnings};
use runestick::unit::LinkerErrors;
use runestick::{Context, Source, Span, Unit};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Load the given path into the runtime.
///
/// The name of the loaded source will be the path as a string.
pub fn load_path(
    context: &Context,
    options: &Options,
    warnings: &mut Warnings,
    path: &Path,
) -> Result<Unit, LoadError> {
    let source = fs::read_to_string(path).map_err(|error| {
        LoadError::from(LoadErrorKind::ReadFile {
            error,
            path: path.to_owned(),
        })
    })?;

    let name = path.display().to_string();
    let unit = load_source(context, options, warnings, Source::new(name, source))?;
    Ok(unit)
}

/// Load the given source and return a number corresponding to its file id.
///
/// Use the provided `name` when generating diagnostics to reference the
/// file.
pub fn load_source(
    context: &Context,
    options: &Options,
    warnings: &mut Warnings,
    code_source: Source,
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
