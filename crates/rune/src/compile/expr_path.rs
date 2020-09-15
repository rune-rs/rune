use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Spanned as _};

/// Compile `self`.
impl Compile<(&ast::Path, Needs)> for Compiler<'_> {
    fn compile(&mut self, (path, needs): (&ast::Path, Needs)) -> CompileResult<()> {
        let span = path.span();
        log::trace!("Path => {:?}", self.source.source(span));

        // NB: do nothing if we don't need a value.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
        }

        let item = self.convert_path_to_item(path)?;

        if let Needs::Value = needs {
            if let Some(local) = item.as_local() {
                if let Some(var) =
                    self.scopes
                        .try_get_var(local, self.source.url(), self.visitor, span)
                {
                    var.copy(&mut self.asm, span, format!("var `{}`", local));
                    return Ok(());
                }
            }
        }

        let meta = match self.lookup_meta(&item, span)? {
            Some(meta) => meta,
            None => {
                let error = match (needs, item.as_local()) {
                    (Needs::Value, Some(local)) => {
                        // light heuristics, treat it as a type error in case the
                        // first character is uppercase.
                        if local.starts_with(char::is_uppercase) {
                            CompileError::new(span, CompileErrorKind::MissingType { item })
                        } else {
                            CompileError::new(
                                span,
                                CompileErrorKind::MissingLocal {
                                    name: local.to_owned(),
                                },
                            )
                        }
                    }
                    _ => CompileError::new(span, CompileErrorKind::MissingType { item }),
                };

                return Err(error);
            }
        };

        self.compile_meta(&meta, span, needs)?;
        Ok(())
    }
}
