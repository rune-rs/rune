/// Indexing for local declarations.
use crate::ast;
use crate::ast::Spanned;
use crate::compile::CompileResult;
use crate::indexing::Indexer;
use crate::parse::Resolve;
use rune_macros::__instrument_ast as instrument;

#[instrument]
pub(crate) fn pat(ast: &mut ast::Pat, idx: &mut Indexer<'_>) -> CompileResult<()> {
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

#[instrument]
fn pat_path(ast: &mut ast::PatPath, idx: &mut Indexer<'_>) -> CompileResult<()> {
    path(&mut ast.path, idx)?;
    Ok(())
}

#[instrument]
fn path(ast: &mut ast::Path, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let id = idx
        .q
        .insert_path(&idx.mod_item, idx.impl_item.as_ref(), &*idx.items.item());
    ast.id.set(id);

    if let Some(i) = ast.try_as_ident_mut() {
        ident(i, idx)?;
    }

    Ok(())
}

#[instrument]
fn ident(ast: &mut ast::Ident, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    let ident = ast.resolve(resolve_context!(idx.q))?;
    idx.scopes.declare(ident.as_ref(), span)?;
    Ok(())
}

#[instrument]
fn pat_object(ast: &mut ast::PatObject, idx: &mut Indexer<'_>) -> CompileResult<()> {
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

#[instrument]
fn pat_vec(ast: &mut ast::PatVec, idx: &mut Indexer<'_>) -> CompileResult<()> {
    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

#[instrument]
fn pat_tuple(ast: &mut ast::PatTuple, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(p) = &mut ast.path {
        path(p, idx)?;
    }

    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

#[instrument]
fn pat_binding(ast: &mut ast::PatBinding, idx: &mut Indexer<'_>) -> CompileResult<()> {
    pat(&mut ast.pat, idx)?;
    Ok(())
}
