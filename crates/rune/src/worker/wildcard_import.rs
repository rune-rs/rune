use crate::meta::CompileMod;
use crate::query::Query;
use crate::{
    CompileError, CompileErrorKind, CompileResult, Context, Item, SourceId, Span, Visibility,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct WildcardImport {
    pub(crate) visibility: Visibility,
    pub(crate) from: Item,
    pub(crate) name: Item,
    pub(crate) source_id: SourceId,
    pub(crate) span: Span,
    pub(crate) module: Arc<CompileMod>,
    pub(crate) found: bool,
}

impl WildcardImport {
    pub(crate) fn process_global(
        &mut self,
        query: &mut Query,
        context: &Context,
    ) -> CompileResult<()> {
        if context.contains_prefix(&self.name) {
            for c in context.iter_components(&self.name) {
                let name = self.name.extended(c);

                query.insert_import(
                    self.source_id,
                    self.span,
                    &self.module,
                    self.visibility,
                    self.from.clone(),
                    name,
                    None::<&str>,
                    true,
                )?;
            }

            self.found = true;
        }

        Ok(())
    }

    pub(crate) fn process_local(mut self, query: &mut Query) -> CompileResult<()> {
        if query.contains_prefix(&self.name) {
            for c in query.iter_components(&self.name) {
                let name = self.name.extended(c);

                query.insert_import(
                    self.source_id,
                    self.span,
                    &self.module,
                    self.visibility,
                    self.from.clone(),
                    name,
                    None::<&str>,
                    true,
                )?;
            }

            self.found = true;
        }

        if !self.found {
            return Err(CompileError::new(
                self.span,
                CompileErrorKind::MissingItem { item: self.name },
            ));
        }

        Ok(())
    }
}
