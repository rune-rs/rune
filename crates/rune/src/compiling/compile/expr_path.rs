use crate::compiling::compile::prelude::*;

/// Compile `self`.
impl Compile2 for ast::Path {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Path => {:?}", c.source.source(span));

        if let Some(ast::PathKind::SelfValue) = self.as_kind() {
            let var = c.scopes.get_var("self", c.source_id, c.visitor, span)?;

            if !needs.value() {
                return Ok(());
            }

            var.copy(&mut c.asm, span, "self");
            return Ok(());
        }

        // NB: do nothing if we don't need a value.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
        }

        let named = c.convert_path_to_named(self)?;

        if let Needs::Value = needs {
            if let Some(local) = named.as_local() {
                if let Some(var) = c.scopes.try_get_var(local, c.source_id, c.visitor, span) {
                    var.copy(&mut c.asm, span, format!("var `{}`", local));
                    return Ok(());
                }
            }
        }

        let meta = match c.lookup_meta(span, &named)? {
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

        c.compile_meta(&meta, span, needs)?;
        Ok(())
    }
}
