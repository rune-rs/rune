use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::{traits::Compile, CompileError};

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
                        .try_get_var(local, self.source.url(), self.visitor, span)?
                {
                    var.copy(&mut self.asm, span, format!("var `{}`", local));
                    return Ok(());
                }
            }
        }

        let meta = match self.lookup_meta(&item, span)? {
            Some(meta) => meta,
            None => match (needs, item.as_local()) {
                (Needs::Value, Some(local)) => {
                    return Err(CompileError::MissingLocal {
                        name: local.to_owned(),
                        span,
                    });
                }
                _ => {
                    return Err(CompileError::MissingType { span, item });
                }
            },
        };

        self.compile_meta(&meta, span, needs)?;
        Ok(())
    }
}
