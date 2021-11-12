/// Indexing for local declarations.
use crate::ast;
use crate::compile::CompileResult;
use crate::indexing::Indexer;
use crate::parse::Resolve;
use crate::Spanned;

pub(crate) trait IndexLocal {
    /// Walk the current type with the given item.
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()>;
}

impl IndexLocal for ast::Pat {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Pat => {:?}", idx.q.sources.source(idx.source_id, span));

        match self {
            ast::Pat::PatPath(pat_path) => {
                pat_path.index_local(idx)?;
            }
            ast::Pat::PatObject(pat_object) => {
                pat_object.index_local(idx)?;
            }
            ast::Pat::PatVec(pat_vec) => {
                pat_vec.index_local(idx)?;
            }
            ast::Pat::PatTuple(pat_tuple) => {
                pat_tuple.index_local(idx)?;
            }
            ast::Pat::PatBinding(pat_binding) => {
                pat_binding.index_local(idx)?;
            }
            ast::Pat::PatIgnore(..) => (),
            ast::Pat::PatLit(..) => (),
            ast::Pat::PatRest(..) => (),
        }

        Ok(())
    }
}

impl IndexLocal for ast::PatPath {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));
        self.path.index_local(idx)?;
        Ok(())
    }
}

impl IndexLocal for ast::Path {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));

        let id = idx
            .q
            .insert_path(&idx.mod_item, idx.impl_item.as_ref(), &*idx.items.item());
        self.id = Some(id);

        if let Some(ident) = self.try_as_ident_mut() {
            ident.index_local(idx)?;
        }

        Ok(())
    }
}

impl IndexLocal for ast::Ident {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));

        let span = self.span();
        let ident = self.resolve(idx.q.storage(), idx.q.sources)?;
        idx.scopes.declare(ident.as_ref(), span)?;
        Ok(())
    }
}

impl IndexLocal for ast::PatObject {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!(
            "PatObject => {:?}",
            idx.q.sources.source(idx.source_id, span)
        );

        match &mut self.ident {
            ast::ObjectIdent::Anonymous(_) => {}
            ast::ObjectIdent::Named(path) => {
                path.index_local(idx)?;
            }
        }

        for (pat, _) in &mut self.items {
            pat.index_local(idx)?;
        }

        Ok(())
    }
}

impl IndexLocal for ast::PatVec {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("PatVec => {:?}", idx.q.sources.source(idx.source_id, span));

        for (pat, _) in &mut self.items {
            pat.index_local(idx)?;
        }

        Ok(())
    }
}

impl IndexLocal for ast::PatTuple {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!(
            "PatTuple => {:?}",
            idx.q.sources.source(idx.source_id, span)
        );

        if let Some(path) = &mut self.path {
            path.index_local(idx)?;
        }

        for (pat, _) in &mut self.items {
            pat.index_local(idx)?;
        }

        Ok(())
    }
}

impl IndexLocal for ast::PatBinding {
    fn index_local(&mut self, idx: &mut Indexer<'_, '_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!(
            "PatBinding => {:?}",
            idx.q.sources.source(idx.source_id, span)
        );
        self.pat.index_local(idx)?;
        Ok(())
    }
}
