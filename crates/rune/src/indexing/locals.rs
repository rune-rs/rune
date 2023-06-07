/// Indexing for local declarations.
use crate::ast::{self, Spanned};
use crate::compile;
use crate::indexing::Indexer;
use crate::parse::Resolve;

use rune_macros::instrument;

#[instrument]
pub(crate) fn pat(ast: &mut ast::Pat, idx: &mut Indexer<'_>) -> compile::Result<()> {
    match ast {
        ast::Pat::Path(p) => {
            pat_path(p, idx)?;
        }
        ast::Pat::Object(p) => {
            pat_object(p, idx)?;
        }
        ast::Pat::Vec(p) => {
            pat_vec(p, idx)?;
        }
        ast::Pat::Tuple(p) => {
            pat_tuple(p, idx)?;
        }
        ast::Pat::Binding(p) => {
            pat_binding(p, idx)?;
        }
        ast::Pat::Ignore(..) => (),
        ast::Pat::Lit(..) => (),
        ast::Pat::Rest(..) => (),
    }

    Ok(())
}

#[instrument]
fn pat_path(ast: &mut ast::PatPath, idx: &mut Indexer<'_>) -> compile::Result<()> {
    path(&mut ast.path, idx)?;
    Ok(())
}

#[instrument]
fn path(ast: &mut ast::Path, idx: &mut Indexer<'_>) -> compile::Result<()> {
    let id = idx
        .q
        .insert_path(idx.mod_item, idx.impl_item, &idx.items.item());
    ast.id.set(id);

    if let Some(i) = ast.try_as_ident_mut() {
        ident(i, idx)?;
    }

    Ok(())
}

#[instrument]
fn ident(ast: &mut ast::Ident, idx: &mut Indexer<'_>) -> compile::Result<()> {
    let span = ast.span();
    let ident = ast.resolve(resolve_context!(idx.q))?;
    idx.scopes.declare(ident.as_ref(), span)?;
    Ok(())
}

#[instrument]
fn pat_object(ast: &mut ast::PatObject, idx: &mut Indexer<'_>) -> compile::Result<()> {
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
fn pat_vec(ast: &mut ast::PatVec, idx: &mut Indexer<'_>) -> compile::Result<()> {
    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

#[instrument]
fn pat_tuple(ast: &mut ast::PatTuple, idx: &mut Indexer<'_>) -> compile::Result<()> {
    if let Some(p) = &mut ast.path {
        path(p, idx)?;
    }

    for (p, _) in &mut ast.items {
        pat(p, idx)?;
    }

    Ok(())
}

#[instrument]
fn pat_binding(ast: &mut ast::PatBinding, idx: &mut Indexer<'_>) -> compile::Result<()> {
    pat(&mut ast.pat, idx)?;
    Ok(())
}
