use crate::compiling::compile::prelude::*;

/// Compile `self`.
impl Compile<(&ast::Path, Needs)> for Compiler<'_> {
    fn compile(&mut self, (path, needs): (&ast::Path, Needs)) -> CompileResult<()> {
        let span = path.span();
        log::trace!("Path => {:?}", self.source.source(span));

        if let Some(ast::PathKind::SelfValue) = path.as_kind() {
            let var = self
                .scopes
                .get_var("self", self.source_id, self.visitor, span)?;

            if !needs.value() {
                return Ok(());
            }

            var.copy(&mut self.asm, span, "self");
            return Ok(());
        }

        // NB: do nothing if we don't need a value.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
        }

        let named = self.convert_path_to_named(path)?;

        if let Needs::Value = needs {
            if let Some(local) = named.as_local() {
                if let Some(var) =
                    self.scopes
                        .try_get_var(local, self.source_id, self.visitor, span)
                {
                    var.copy(&mut self.asm, span, format!("var `{}`", local));
                    return Ok(());
                }
            }
        }

        let meta = match self.lookup_meta(span, &named)? {
            Some(meta) => meta,
            None => {
                let error = match (needs, named.as_local()) {
                    (Needs::Value, Some(local)) => {
                        // light heuristics, treat it as a type error in case the
                        // first character is uppercase.
                        if local.starts_with(char::is_uppercase) {
                            CompileError::new(
                                span,
                                CompileErrorKind::MissingType {
                                    item: named.item.clone(),
                                },
                            )
                        } else {
                            CompileError::new(
                                span,
                                CompileErrorKind::MissingLocal {
                                    name: local.to_owned(),
                                },
                            )
                        }
                    }
                    _ => CompileError::new(
                        span,
                        CompileErrorKind::MissingType {
                            item: named.item.clone(),
                        },
                    ),
                };

                return Err(error);
            }
        };

        self.compile_meta(&meta, span, needs)?;
        Ok(())
    }
}
