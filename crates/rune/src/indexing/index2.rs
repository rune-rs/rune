//! Indexing step which validates the AST and indexes declarations.

use tracing::instrument_ast;

use crate::ast::{self, Kind};
use crate::compile::{Result, WithSpan};
use crate::grammar::{object_key, Remaining, Stream};
use crate::parse::Resolve;

use super::Indexer;

use Kind::*;

#[instrument_ast(span = p)]
pub(crate) fn stmt(cx: &mut Indexer<'_, '_>, p: &mut Stream) -> Result<()> {
    match p.kind() {
        Local => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // local(cx, p)?;
        }
        ItemStruct => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // item_struct(cx, p)?;
        }
        ItemEnum => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // item_enum(cx, p)?;
        }
        ItemFn => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            item_fn(cx, p)?;
        }
        ItemUse => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // item_use(cx, p)?;
        }
        ItemImpl => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // item_impl(cx, p)?;
        }
        ItemMod => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            item_mod(cx, p)?;
        }
        ItemConst => {
            attributes(cx, p)?;
            modifiers(cx, p)?;
            // item_const(cx, p)?;
        }
        _ => {
            expr(cx, p)?;
        }
    }

    Ok(())
}

fn expr(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<Kind> {
    match p.kind() {
        Expr => {
            let _attrs = attributes(cx, p)?;
            let _mods = modifiers(cx, p)?;
            expr_labels(cx, p)?;
            return p.pump()?.parse(|p| expr(cx, p));
        }
        TemplateString => {
            p.pump()?;
        }
        ExprMacroCall => {}
        ExprLit => {
            p.pump()?;
        }
        Block => {
            block(cx, p)?;
        }
        ExprAssign => {
            p.pump()?.parse(|p| expr(cx, p))?;
            p.expect(K![=])?;
            p.pump()?.parse(|p| expr(cx, p))?;
        }
        ExprPath => {
            p.expect(Path)?.parse(|p| path(cx, p))?;
        }
        ExprArray => {
            exprs(cx, p, K!['['], K![']'])?;
        }
        ExprTuple => {
            exprs(cx, p, K!['('], K![')'])?;
        }
        ExprObject => {
            expr_object(cx, p)?;
        }
        ExprBinary => {
            p.pump()?.parse(|p| expr(cx, p))?;

            while p.try_pump(ExprOperator)?.is_some() {
                p.pump()?.parse(|p| expr(cx, p))?;
            }
        }
        ExprUnary => {
            p.pump()?;
            p.pump()?.parse(|p| expr(cx, p))?;
        }
        ExprGroup => {
            p.expect(K!['('])?;
            p.pump()?.parse(|p| expr(cx, p))?;
            p.expect(K![')'])?;
        }
        ExprIf => {
            expr_if(cx, p)?;
        }
        ExprWhile => {
            expr_while(cx, p)?;
        }
        ExprLoop => {
            expr_loop(cx, p)?;
        }
        ExprBreak => {
            p.expect(K![break])?;

            while matches!(p.peek(), K!['label]) {
                p.pump()?;
            }

            if let Some(node) = p.try_pump(Expr)? {
                node.parse(|p| expr(cx, p))?;
            }
        }
        ExprContinue => {
            p.expect(K![continue])?;

            let mut has_label = false;

            while matches!(p.peek(), K!['label]) {
                let label = p.pump()?;

                if has_label {
                    cx.error(label.msg("more than one label in continue"))?;
                }

                has_label = true;
            }
        }
        ExprReturn => {
            p.expect(K![return])?;

            if let Some(node) = p.try_pump(Expr)? {
                node.parse(|p| expr(cx, p))?;
            }
        }
        ExprYield => {
            let l = cx.scopes.mark().with_span(&*p)?;
            l.yields.try_push(p.span())?;
            p.expect(K![yield])?;

            if let Some(node) = p.try_pump(Expr)? {
                node.parse(|p| expr(cx, p))?;
            }
        }
        ExprFor => {
            expr_for(cx, p)?;
        }
        ExprMatch => {
            expr_match(cx, p)?;
        }
        ExprSelect => {
            expr_select(cx, p)?;
        }
        ExprRangeFull => {
            p.pump()?;
        }
        ExprRangeFrom => {
            p.pump()?.parse(|p| expr(cx, p))?;
            p.pump()?;
        }
        ExprRangeTo | ExprRangeToInclusive => {
            p.pump()?;
            p.pump()?.parse(|p| expr(cx, p))?;
        }
        ExprRange | ExprRangeInclusive => {
            p.pump()?.parse(|p| expr(cx, p))?;
            p.pump()?;
            p.pump()?.parse(|p| expr(cx, p))?;
        }
        ExprClosure => {
            expr_closure(cx, p)?;
        }
        ExprChain => {
            expr_chain(cx, p)?;
        }
        _ => {
            return Err(p.unsupported("expression"));
        }
    }

    Ok(p.kind())
}

fn exprs(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>, open: Kind, close: Kind) -> Result<()> {
    p.expect(open)?;

    while let Some(node) = p.try_pump(Expr)? {
        node.parse(|p| expr(cx, p))?;
        p.remaining(cx, K![,])?.at_most_one(cx)?;
    }

    p.one(close)?.one(cx)?;
    Ok(())
}

fn pats(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>, open: Kind, close: Kind) -> Result<()> {
    p.expect(open)?;

    while let Some(node) = p.try_pump(Pat)? {
        node.parse(|p| pat(cx, p))?;
        p.remaining(cx, K![,])?.at_most_one(cx)?;
    }

    p.one(close)?.one(cx)?;
    Ok(())
}

fn expr_object(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            p.expect(AnonymousObjectKey)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(cx, p))?;
        }
    }

    p.expect(K!['{'])?;

    let mut comma = Remaining::default();
    let mut empty = true;

    while matches!(p.peek(), object_key!()) {
        if !empty {
            comma.one(cx)?;
        }

        p.pump()?;

        if p.try_pump(K![:])?.is_some() {
            p.pump()?.parse(|p| expr(cx, p))?;
        }

        comma = p.remaining(cx, K![,])?;
        empty = false;
    }

    comma.at_most_one(cx)?;
    p.remaining(cx, K!['}'])?.one(cx)?;
    Ok(())
}

fn expr_select(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![select])?;
    p.expect(K!['{'])?;

    while matches!(p.peek(), K![default] | Pat) {
        match p.peek() {
            K![default] => {
                p.expect(K![default])?;
            }
            _ => {
                p.expect(Pat)?.parse(|p| pat(cx, p))?;
                p.expect(K![=])?;
                p.expect(Expr)?.parse(|p| expr(cx, p))?;
            }
        }

        p.one(K![=>])?.one(cx)?;

        let is_block = p.pump()?.parse(|p| {
            let kind = expr(cx, p)?;
            Ok(matches!(kind, Block))
        })?;

        let comma = p.remaining(cx, K![,])?;

        if is_block {
            comma.at_most_one(cx)?;
        } else {
            comma.one(cx)?;
        }
    }

    p.one(K!['}'])?.one(cx)?;
    Ok(())
}

fn expr_labels(_: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    for _ in p.by_ref() {}
    Ok(())
}

fn expr_closure(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    if let Some(node) = p.try_pump(ClosureArguments)? {
        node.parse(|p| {
            if p.try_pump(K![||])?.is_none() {
                p.expect(K![|])?;

                let mut empty = true;
                let mut last_comma = Remaining::default();

                while let Some(node) = p.try_pump(Pat)? {
                    if !empty {
                        last_comma.one(cx)?;
                    }

                    node.parse(|p| pat(cx, p))?;
                    last_comma = p.remaining(cx, K![,])?;
                    empty = false;
                }

                last_comma.at_most_one(cx)?;

                if p.try_pump(K![|])?.is_none() {
                    cx.error(p.msg("missing closing `|`"))?;
                }
            }

            Ok(())
        })?;
    } else {
        cx.error(p.msg("missing closure arguments starting with `|`"))?;
    }

    if let Some(node) = p.try_pump(Expr)? {
        node.parse(|p| expr(cx, p))?;
    } else {
        cx.error(p.msg("missing closure body"))?;
    }

    Ok(())
}

fn expr_chain(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.pump()?.parse(|p| expr(cx, p))?;

    for node in p.by_ref() {
        node.parse(|p| {
            match p.kind() {
                ExprTry => {
                    p.one(K![?])?.one(cx)?;
                }
                ExprAwait => {
                    let l = cx.scopes.mark().with_span(&*p)?;
                    l.awaits.try_push(p.span())?;
                    p.one(K![.])?.one(cx)?;
                    p.one(K![await])?.one(cx)?;
                }
                ExprField => {
                    p.one(K![.])?.one(cx)?;

                    match p.peek() {
                        K![number] => {
                            p.pump()?;
                        }
                        _ => {
                            p.expect(Path)?.parse(|p| path(cx, p))?;
                        }
                    }
                }
                ExprCall => {
                    exprs(cx, p, K!['('], K![')'])?;
                }
                ExprIndex => {
                    p.expect(K!['['])?;
                    p.pump()?.parse(|p| expr(cx, p))?;
                    p.one(K![']'])?.one(cx)?;
                }
                _ => {
                    return Err(p.unsupported("expression chain"));
                }
            }

            Ok(())
        })?;
    }

    Ok(())
}

fn expr_if(o: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(If)?;
    condition_or_expr(o, p)?;

    if let Some(op) = p.try_pump(Block)? {
        op.parse(|p| block(o, p))?;
    } else {
        o.error(p.msg("missing block"))?;
    }

    for node in p.by_ref() {
        match node.kind() {
            ExprElse => {
                node.parse(|p| {
                    p.expect(Else)?;
                    p.expect(Block)?.parse(|p| block(o, p))?;
                    Ok(())
                })?;
            }
            ExprElseIf => {
                node.parse(|p| {
                    p.expect(Else)?;
                    p.expect(If)?;
                    condition_or_expr(o, p)?;
                    p.expect(Block)?.parse(|p| block(o, p))?;
                    Ok(())
                })?;
            }
            _ => {
                return Err(node.msg("unsupported block"));
            }
        }
    }

    Ok(())
}

fn expr_while(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![while])?;
    condition_or_expr(cx, p)?;
    p.expect(Block)?.parse(|p| block(cx, p))?;
    Ok(())
}

fn expr_loop(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![loop])?;
    p.expect(Block)?.parse(|p| block(cx, p))?;
    Ok(())
}

fn expr_for(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![for])?;
    p.expect(Pat)?.parse(|p| pat(cx, p))?;
    p.expect(K![in])?;
    p.pump()?.parse(|p| expr(cx, p))?;
    p.expect(Block)?.parse(|p| block(cx, p))?;
    Ok(())
}

fn expr_match(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![match])?;
    p.pump()?.parse(|p| expr(cx, p))?;

    if p.try_pump(K!['{'])?.is_none() {
        return Err(p.msg("missing match block"));
    };

    while let Some(node) = p.try_pump(Pat)? {
        node.parse(|p| pat(cx, p))?;

        if p.try_pump(K![if])?.is_some() {
            p.pump()?.parse(|p| expr(cx, p))?;
        }

        p.one(K![=>])?.one(cx)?;

        let is_block = p.pump()?.parse(|p| {
            let kind = expr(cx, p)?;
            Ok(matches!(kind, Block))
        })?;

        let comma = p.remaining(cx, K![,])?;

        if is_block {
            comma.at_most_one(cx)?;
        } else {
            comma.one(cx)?;
        }
    }

    p.one(K!['}'])?.one(cx)?;
    Ok(())
}

fn condition_or_expr(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    if let Some(node) = p.try_pump(Condition)? {
        node.parse(|p| condition(cx, p))?;
    } else {
        p.pump()?.parse(|p| expr(cx, p))?;
    }

    Ok(())
}

fn condition(o: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![let])?;
    p.expect(Pat)?.parse(|p| pat(o, p))?;
    p.expect(K![=])?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn item_fn(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![fn])?;

    let guard = if matches!(p.peek(), K![ident]) {
        let ident = p.pump()?.ast::<ast::Ident>()?;
        let name = ident.resolve(resolve_context!(cx.q))?;
        cx.items.push_name(name.as_ref())?
    } else {
        cx.error(p.msg("expected function name"))?;
        cx.items.push_id()?
    };

    if let Some(node) = p.try_pump(FnArgs)? {
        node.parse(|p| pats(cx, p, K!['('], K![')']))?;
    } else {
        cx.error(p.msg("missing function arguments"))?;
    }

    if let Some(node) = p.try_pump(Block)? {
        node.parse(|p| block(cx, p))?;
    } else {
        cx.error(p.msg("missing function block"))?;
    }

    cx.items.pop(guard).with_span(p)?;
    Ok(())
}

fn item_mod(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.expect(K![mod])?;

    let guard = if matches!(p.peek(), K![ident]) {
        let ident = p.pump()?.ast::<ast::Ident>()?;
        let name = ident.resolve(resolve_context!(cx.q))?;
        cx.items.push_name(name.as_ref())?
    } else {
        cx.error(p.msg("expected function name"))?;
        cx.items.push_id()?
    };

    if let Some(node) = p.try_pump(Block)? {
        node.parse(|p| block(cx, p))?;
    } else {
        cx.error(p.msg("missing function block"))?;
    }

    cx.items.pop(guard).with_span(p)?;
    Ok(())
}

fn block(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    p.one(K!['{'])?.one(cx)?;

    cx.capture(|cx| {
        p.expect(BlockBody)?.parse(|p| {
            for node in p.children() {
                node.parse(|p| stmt(cx, p))?;
            }

            Ok(())
        })
    })?;

    p.one(K!['}'])?.one(cx)?;
    Ok(())
}

fn pat(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    let _attrs = attributes(cx, p)?;

    p.pump()?.parse(|p| {
        match p.kind() {
            PatLit => {}
            PatIgnore => {}
            PatRest => {}
            Path => {
                path(cx, p)?;
            }
            PatArray => {
                pats(cx, p, K!['['], K![']'])?;
            }
            PatTuple => {
                pats(cx, p, K!['('], K![')'])?;
            }
            PatObject => {
                pat_object(cx, p)?;
            }
            _ => {
                cx.error(p.unsupported("pattern"))?;
            }
        }

        Ok(())
    })
}

fn pat_object(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            p.expect(AnonymousObjectKey)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(cx, p))?;
        }
    }

    p.expect(K!['{'])?;

    let mut comma = Remaining::default();
    let mut empty = true;

    while matches!(p.peek(), object_key!() | K![..]) {
        if !empty {
            comma.one(cx)?;
        }

        match p.peek() {
            object_key!() => {
                p.pump()?;

                if p.try_pump(K![:])?.is_some() {
                    p.expect(Pat)?.parse(|p| pat(cx, p))?;
                }
            }
            _ => {
                p.expect(K![..])?;
            }
        }

        comma = p.remaining(cx, K![,])?;
        empty = false;
    }

    comma.at_most_one(cx)?;
    p.remaining(cx, K!['}'])?.one(cx)?;
    Ok(())
}

#[derive(Default)]
struct Attrs {
    #[allow(unused)]
    test: bool,
}

#[derive(Default, PartialEq, Eq)]
enum Modifiers {
    #[default]
    Inherit,
    Public,
    Crate,
    InPath,
}

fn attributes(_: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<Attrs> {
    let attrs = Attrs::default();

    while let Some(attr) = p.try_pump(Attribute)? {
        dbg!(attr.kind());
    }

    Ok(attrs)
}

fn modifiers(cx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<Modifiers> {
    let mut modifiers = Modifiers::default();

    let Some(mods) = p.try_pump(Modifiers)? else {
        return Ok(modifiers);
    };

    mods.parse(|p| {
        for node in p.by_ref() {
            match node.kind() {
                ModifierCrate => {
                    if modifiers != Modifiers::Public {
                        cx.error(node.msg("missing `pub` modifier"))?;
                    }

                    cx.capture(|cx| {
                        node.parse(|p| {
                            p.expect(K!['('])?;
                            p.expect(K![crate])?;
                            p.one(K![')'])?.one(cx)?;
                            Ok(())
                        })
                    })?;

                    modifiers = Modifiers::Crate;
                }
                ModifierIn => {
                    if modifiers != Modifiers::Public {
                        cx.error(node.msg("missing `pub` modifier"))?;
                    }

                    cx.capture(|cx| {
                        node.parse(|p| {
                            p.expect(K!['('])?;
                            p.expect(K![in])?;
                            p.expect(Path)?.parse(|p| path(cx, p))?;
                            p.one(K![')'])?.one(cx)?;
                            Ok(())
                        })
                    })?;

                    modifiers = Modifiers::InPath;
                }
                K![pub] => {
                    if modifiers != Modifiers::Inherit {
                        cx.error(node.msg("clashing visibility modifier"))?;
                    }

                    modifiers = Modifiers::Public;
                }
                _ => {
                    cx.error(node.unsupported("modifier"))?;
                }
            }
        }

        Ok(())
    })?;

    Ok(modifiers)
}

fn path(_: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    for node in p.by_ref() {
        match node.kind() {
            PathGenerics => {}
            _ => {}
        }
    }

    Ok(())
}
