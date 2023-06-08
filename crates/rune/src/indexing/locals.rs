/// Indexing for local declarations.
use crate::ast::{self, Spanned};
use crate::compile;
use crate::indexing::Indexer;
use crate::parse::Resolve;

use rune_macros::instrument;

#[instrument(span = ast)]
pub(crate) fn pat(idx: &mut Indexer<'_>, ast: &mut ast::Pat) -> compile::Result<()> {
    match ast {
        ast::Pat::Path(p) => {
            pat_path(idx, p)?;
        }
        ast::Pat::Object(p) => {
            pat_object(idx, p)?;
        }
        ast::Pat::Vec(p) => {
            pat_vec(idx, p)?;
        }
        ast::Pat::Tuple(p) => {
            pat_tuple(idx, p)?;
        }
        ast::Pat::Binding(p) => {
            pat_binding(idx, p)?;
        }
        ast::Pat::Ignore(..) => (),
        ast::Pat::Lit(..) => (),
        ast::Pat::Rest(..) => (),
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_path(idx: &mut Indexer<'_>, ast: &mut ast::PatPath) -> compile::Result<()> {
    path(idx, &mut ast.path)?;
    Ok(())
}

#[instrument(span = ast)]
fn path(idx: &mut Indexer<'_>, ast: &mut ast::Path) -> compile::Result<()> {
    let id = idx
        .q
        .insert_path(idx.mod_item, idx.impl_item, &idx.items.item());
    ast.id.set(id);

    if let Some(i) = ast.try_as_ident_mut() {
        ident(idx, i)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn ident(idx: &mut Indexer<'_>, ast: &mut ast::Ident) -> compile::Result<()> {
    let span = ast.span();
    let ident = ast.resolve(resolve_context!(idx.q))?;
    idx.scopes.declare(ident.as_ref(), span)?;
    Ok(())
}

#[instrument(span = ast)]
fn pat_object(idx: &mut Indexer<'_>, ast: &mut ast::PatObject) -> compile::Result<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Anonymous(_) => {}
        ast::ObjectIdent::Named(p) => {
            path(idx, p)?;
        }
    }

    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_vec(idx: &mut Indexer<'_>, ast: &mut ast::PatVec) -> compile::Result<()> {
    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_tuple(idx: &mut Indexer<'_>, ast: &mut ast::PatTuple) -> compile::Result<()> {
    if let Some(p) = &mut ast.path {
        path(idx, p)?;
    }

    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_binding(idx: &mut Indexer<'_>, ast: &mut ast::PatBinding) -> compile::Result<()> {
    pat(idx, &mut ast.pat)?;
    Ok(())
}
