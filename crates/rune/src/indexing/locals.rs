/// Indexing for local declarations.
use crate::ast;
use crate::ast::Spanned;
use crate::compile::CompileResult;
use crate::indexing::Indexer;
use crate::parse::Resolve;

pub(crate) fn pat(ast: &mut ast::Pat, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!("Pat => {:?}", idx.q.sources.source(idx.source_id, span));

    match ast {
        ast::Pat::PatPath(p) => {
            pat_path(p, idx)?;
        }
        ast::Pat::PatObject(p) => {
            pat_object(p, idx)?;
        }
        ast::Pat::PatVec(p) => {
            pat_vec(p, idx)?;
        }
        ast::Pat::PatTuple(p) => {
            pat_tuple(p, idx)?;
        }
        ast::Pat::PatBinding(p) => {
            pat_binding(p, idx)?;
        }
        ast::Pat::PatIgnore(..) => (),
        ast::Pat::PatLit(..) => (),
        ast::Pat::PatRest(..) => (),
    }

    Ok(())
}

fn pat_path(ast: &mut ast::PatPath, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));
    path(&mut ast.path, idx)?;
    Ok(())
}

fn path(ast: &mut ast::Path, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));

    let id = idx
        .q
        .insert_path(&idx.mod_item, idx.impl_item.as_ref(), &*idx.items.item());
    ast.id = Some(id);

    if let Some(i) = ast.try_as_ident_mut() {
        ident(i, idx)?;
    }

    Ok(())
}

fn ident(ast: &mut ast::Ident, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!("Ident => {:?}", idx.q.sources.source(idx.source_id, span));

    let span = ast.span();
    let ident = ast.resolve(idx.q.storage(), idx.q.sources)?;
    idx.scopes.declare(ident.as_ref(), span)?;
    Ok(())
}

fn pat_object(ast: &mut ast::PatObject, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!(
        "PatObject => {:?}",
        idx.q.sources.source(idx.source_id, span)
    );

    match &mut ast.ident {
        ast::ObjectIdent::Anonymous(_) => {}
        ast::ObjectIdent::Named(p) => {
            path(p, idx)?;
        }
    }

    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

fn pat_vec(ast: &mut ast::PatVec, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!("PatVec => {:?}", idx.q.sources.source(idx.source_id, span));

    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

fn pat_tuple(ast: &mut ast::PatTuple, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!(
        "PatTuple => {:?}",
        idx.q.sources.source(idx.source_id, span)
    );

    if let Some(p) = &mut ast.path {
        path(p, idx)?;
    }

    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

fn pat_binding(ast: &mut ast::PatBinding, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    log::trace!(
        "PatBinding => {:?}",
        idx.q.sources.source(idx.source_id, span)
    );
    pat(&mut ast.pat, idx)?;
    Ok(())
}
