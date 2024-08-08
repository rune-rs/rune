use core::mem::take;

use crate::ast::Kind;
use crate::compile::Result;

use super::{Comments, Node, Output, Remaining, Stream, Tree};

type ParserFn<'a> = fn(&mut Output<'a>, p: &mut Stream<'a>) -> Result<()>;

use Comments::*;
use Kind::*;

#[derive(Default)]
struct Attrs {
    skip: bool,
}

/// Test if a node is the `#[runefmt::skip]` attribute.
fn is_runefmt_skip<'a>(o: &Output<'a>, node: Node<'a>) -> bool {
    let mut skip = None;

    _ = node.parse(|p| {
        p.expect(K![#])?;

        p.expect(K!['['])?;

        p.expect(TokenStream)?.parse(|p| {
            let ns = p.pump()?;
            p.expect(K![::])?;
            let name = p.pump()?;

            skip = skip.or(
                match (o.source.get(ns.span())?, o.source.get(name.span())?) {
                    ("runefmt", "skip") => Some(true),
                    _ => None,
                },
            );

            Ok(())
        })?;

        p.expect(K![']'])?;
        Ok(())
    });

    skip.unwrap_or(false)
}

pub(super) fn root<'a>(o: &mut Output<'a>, tree: &'a Tree) -> Result<()> {
    tree.parse(|p| block_content(o, p))?;
    o.nl(1)?;
    Ok(())
}

fn expr_labels<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    while let Some(label) = p.next() {
        o.write(label)?;
        p.remaining(o, K![:])?.write(o)?;
        o.ws()?;
    }

    Ok(())
}

fn attributes<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    if let Some(n) = p.try_pump(Attributes)? {
        for attr in n.into_stream() {
            attrs.skip |= is_runefmt_skip(o, attr.clone());
            o.write(attr)?;
            o.nl(1)?;
        }
    }

    Ok(attrs)
}

fn modifiers<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    if let Some(mods) = p.try_pump(Modifiers)? {
        mods.parse(|p| {
            let mut any = false;

            for node in p.by_ref() {
                match node.kind() {
                    ModifierCrate => {
                        node.parse(|p| {
                            o.write(p.expect(K!['('])?)?;
                            o.write(p.expect(K![crate])?)?;
                            p.one(K![')'])?.write(o)?;
                            Ok(())
                        })?;
                    }
                    ModifierIn => {
                        node.parse(|p| {
                            o.write(p.expect(K!['('])?)?;
                            o.write(p.expect(K![in])?)?;
                            o.ws()?;
                            p.expect(Path)?.parse(|p| path(o, p))?;
                            p.one(K![')'])?.write(o)?;
                            Ok(())
                        })?;
                    }
                    Error => {
                        return Err(p.unsupported("modifier"));
                    }
                    _ => {
                        if any {
                            o.ws()?;
                        }

                        o.write(node)?;
                    }
                }

                any = true;
            }

            if any {
                o.ws()?;
            }

            Ok(())
        })?;
    }

    Ok(())
}

fn stmt<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<(Kind, bool)> {
    let mut needs_semi = false;

    match p.kind() {
        Local => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                local(o, p)?;
            }

            needs_semi = true;
        }
        ItemStruct => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                needs_semi = item_struct(o, p)?;
            }
        }
        ItemEnum => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                item_enum(o, p)?;
            }
        }
        ItemFn => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                item_fn(o, p)?;
            }
        }
        ItemUse => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                item_use(o, p)?;
            }
        }
        ItemImpl => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                item_impl(o, p)?;
            }
        }
        ItemMod => {
            if attributes(o, p)?.skip {
                p.write_remaining(o)?;
            } else {
                modifiers(o, p)?;
                needs_semi = item_mod(o, p)?;
            }
        }
        ItemConst => {
            attributes(o, p)?;
            modifiers(o, p)?;
            item_const(o, p)?;
            needs_semi = true;
        }
        _ => {
            let kind = expr_with_kind(o, p)?;
            return Ok((kind, false));
        }
    }

    Ok((p.kind(), needs_semi))
}

fn local<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![let])?)?;
    o.ws()?;
    p.expect(Pat)?.parse(|p| pat(o, p))?;
    o.ws()?;
    p.one(K![=])?.write(o)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn pat<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut attrs = Attrs::default();

    if let Some(n) = p.try_pump(Attributes)? {
        for attr in n.into_stream() {
            attrs.skip |= is_runefmt_skip(o, attr.clone());
            o.write(attr)?;
            o.ws()?;
        }
    }

    p.pump()?.parse(|p| {
        match p.kind() {
            PatLit => {
                o.write(p.pump()?)?;
            }
            PatIgnore => {
                o.write(p.pump()?)?;
            }
            PatRest => {
                o.write(p.pump()?)?;
            }
            Path => {
                path(o, p)?;
            }
            PatArray => {
                pat_array(o, p)?;
            }
            PatTuple => {
                let trailing = if let Some(node) = p.try_pump(Path)? {
                    node.parse(|p| path(o, p))?;
                    false
                } else {
                    true
                };

                tuple(o, p, Pat, pat, trailing)?;
            }
            PatObject => {
                pat_object(o, p)?;
            }
            _ => {
                return Err(p.unsupported("pattern"));
            }
        }

        Ok(())
    })
}

fn path<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    for node in p.by_ref() {
        match node.kind() {
            PathGenerics => {
                node.parse(|p| path_generics(o, p))?;
            }
            _ => {
                o.write(node)?;
            }
        }
    }

    Ok(())
}

fn path_generics<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![<])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(Path)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| path(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    }

    p.one(K![>])?.write(o)?;
    Ok(())
}

fn pat_array<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['['])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(Pat)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| pat(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    }

    p.one(K![']'])?.write(o)?;
    Ok(())
}

fn tuple<'a>(
    o: &mut Output<'a>,
    p: &mut Stream<'a>,
    kind: Kind,
    parser: ParserFn<'a>,
    trailing: bool,
) -> Result<()> {
    o.write(p.expect(K!['('])?)?;

    let mut count = 0usize;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(kind)? {
        o.comments(Prefix)?;

        if count > 0 {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| parser(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        count += 1;
        o.comments(Suffix)?;
    }

    if count == 1 && trailing {
        last_comma.write(o)?;
    } else {
        last_comma.ignore(o)?;

        if count == 0 {
            o.comments(Infix)?;
        }
    }

    p.one(K![')'])?.write(o)?;
    Ok(())
}

fn expr_object<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            o.write(p.expect(AnonymousObjectKey)?)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(o, p))?;
            o.ws()?;
        }
    }

    let mut count = 0;
    let mut expanded = o.source.is_at_least(p.span(), 80)?;

    for node in p.children() {
        if expanded {
            break;
        }

        count += usize::from(matches!(node.kind(), object_key!()));
        expanded |= matches!(node.kind(), Kind::Comment) || count >= 6;
    }

    if expanded {
        expr_object_loose(o, p)
    } else {
        expr_object_compact(o, p)
    }
}

fn expr_object_loose<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['{'])?)?;

    o.nl(1)?;
    o.indent(1)?;

    while matches!(p.peek(), object_key!()) {
        o.comments(Line)?;
        o.write(p.pump()?)?;

        if let Some(colon) = p.try_pump(K![:])? {
            o.write(colon)?;
            o.ws()?;
            p.pump()?.parse(|p| expr(o, p))?;
        }

        p.remaining(o, K![,])?.write(o)?;
        o.nl(1)?;
    }

    o.nl(1)?;
    o.indent(-1)?;

    p.remaining(o, K!['}'])?.write(o)?;
    Ok(())
}

fn expr_object_compact<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['{'])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while matches!(p.peek(), object_key!()) {
        if !empty {
            last_comma.write(o)?;
        }

        o.ws()?;

        o.write(p.pump()?)?;

        if let Some(colon) = p.try_pump(K![:])? {
            o.write(colon)?;
            o.ws()?;
            p.pump()?.parse(|p| expr(o, p))?;
        }

        last_comma = p.remaining(o, K![,])?;
        empty = false;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    } else {
        o.ws()?;
    }

    p.remaining(o, K!['}'])?.write(o)?;
    Ok(())
}

fn pat_object<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            o.write(p.expect(AnonymousObjectKey)?)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(o, p))?;
            o.ws()?;
        }
    }

    o.write(p.expect(K!['{'])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while matches!(p.peek(), object_key!() | K![..]) {
        if !empty {
            last_comma.write(o)?;
        }

        o.ws()?;

        match p.peek() {
            object_key!() => {
                o.write(p.pump()?)?;

                if let Some(colon) = p.try_pump(K![:])? {
                    o.write(colon)?;
                    o.ws()?;
                    p.expect(Pat)?.parse(|p| pat(o, p))?;
                }
            }
            _ => {
                o.write(p.expect(K![..])?)?;
            }
        }

        last_comma = p.remaining(o, K![,])?;
        empty = false;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    } else {
        o.ws()?;
    }

    p.remaining(o, K!['}'])?.write(o)?;
    Ok(())
}

fn expr<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    expr_with_kind(o, p)?;
    Ok(())
}

fn expr_with_kind<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<Kind> {
    match p.kind() {
        Expr => {
            let mut attrs = Attrs::default();

            if let Some(n) = p.try_pump(Attributes)? {
                for attr in n.into_stream() {
                    attrs.skip |= is_runefmt_skip(o, attr.clone());
                    o.write(attr)?;
                    o.ws()?;
                }
            }

            if attrs.skip {
                p.write_remaining(o)?;
                return Ok(Expr);
            } else {
                modifiers(o, p)?;

                if let Some(label) = p.try_pump(Labels)? {
                    label.parse(|p| expr_labels(o, p))?;
                }

                return p.pump()?.parse(|p| expr_with_kind(o, p));
            }
        }
        TemplateString => {
            o.write(p.pump()?)?;
        }
        ExprLit => {
            o.write(p.pump()?)?;
        }
        Block => {
            block_with(o, p, true)?;
        }
        ExprAssign => {
            expr_assign(o, p)?;
        }
        ExprPath => {
            p.expect(Path)?.parse(|p| path(o, p))?;
        }
        ExprArray => {
            exprs(o, p, K!['['], K![']'])?;
        }
        ExprTuple => {
            tuple(o, p, Expr, expr, true)?;
        }
        ExprObject => {
            expr_object(o, p)?;
        }
        ExprBinary => {
            expr_binary(o, p)?;
        }
        ExprUnary => {
            expr_unary(o, p)?;
        }
        ExprGroup => {
            o.write(p.expect(K!['('])?)?;

            let mut empty = true;

            if let Some(node) = p.try_pump(Expr)? {
                o.comments(Prefix)?;
                node.parse(|p| expr(o, p))?;
                o.comments(Suffix)?;
                empty = false;
            }

            if empty {
                o.comments(Infix)?;
            }

            p.one(K![')'])?.write(o)?;
        }
        ExprIf => {
            expr_if(o, p)?;
        }
        ExprWhile => {
            expr_while(o, p)?;
        }
        ExprLoop => {
            expr_loop(o, p)?;
        }
        ExprBreak => {
            expr_break(o, p)?;
        }
        ExprContinue => {
            expr_continue(o, p)?;
        }
        ExprReturn => {
            expr_return(o, p)?;
        }
        ExprYield => {
            expr_yield(o, p)?;
        }
        ExprFor => {
            expr_for(o, p)?;
        }
        ExprMatch => {
            expr_match(o, p)?;
        }
        ExprSelect => {
            expr_select(o, p)?;
        }
        ExprRangeFull => {
            o.write(p.pump()?)?;
        }
        ExprRangeFrom => {
            p.pump()?.parse(|p| expr(o, p))?;
            o.write(p.pump()?)?;
        }
        ExprRangeTo | ExprRangeToInclusive => {
            o.write(p.pump()?)?;
            p.pump()?.parse(|p| expr(o, p))?;
        }
        ExprRange | ExprRangeInclusive => {
            p.pump()?.parse(|p| expr(o, p))?;
            o.write(p.pump()?)?;
            p.pump()?.parse(|p| expr(o, p))?;
        }
        ExprClosure => {
            expr_closure(o, p)?;
        }
        ExprChain => {
            expr_chain(o, p)?;
        }
        Error => {
            if o.options.error_recovery {
                p.write_remaining_trimmed(o)?;
            } else {
                return Err(p.unsupported("expression"));
            }
        }
        _ => {
            return Err(p.unsupported("expression"));
        }
    }

    Ok(p.kind())
}

fn loose_expr_macro_call<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['{'])?)?;

    p.expect(TokenStream)?.parse(|p| {
        if p.is_eof() {
            return Ok(());
        }

        o.nl(1)?;
        o.indent(1)?;

        let mut buf = None;
        let mut has_ws = false;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                o.write_raw(node)?;
                o.nl(1)?;
                has_ws = true;
                continue;
            }

            if node.is_whitespace() {
                buf = Some(node);
                continue;
            }

            if let Some(buf) = buf.take() {
                if !has_ws {
                    o.write_raw(buf)?;
                }
            }

            o.flush_whitespace(false)?;
            o.write_raw(node)?;
            has_ws = false;
        }

        o.nl(1)?;
        o.indent(-1)?;
        Ok(())
    })?;

    o.write(p.expect(K!['}'])?)?;
    Ok(())
}

fn compact_expr_macro_call<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['('])?)?;

    p.expect(TokenStream)?.parse(|p| {
        let mut buf = None;
        let mut has_ws = false;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                o.write_raw(node)?;
                o.ws()?;
                has_ws = true;
                continue;
            }

            if node.is_whitespace() {
                buf = Some(node);
                continue;
            }

            if let Some(buf) = buf.take() {
                if !has_ws {
                    o.write_raw(buf)?;
                }
            }

            o.flush_whitespace(false)?;
            o.write_raw(node)?;
            has_ws = false;
        }

        Ok(())
    })?;

    o.write(p.expect(K![')'])?)?;
    Ok(())
}

fn expr_assign<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.pump()?.parse(|p| expr(o, p))?;
    o.ws()?;
    o.write(p.expect(K![=])?)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn exprs<'a>(o: &mut Output<'a>, p: &mut Stream<'a>, open: Kind, close: Kind) -> Result<()> {
    let mut count = 0;
    let mut expanded = o.source.is_at_least(p.span(), 80)?;

    for node in p.children() {
        if expanded {
            break;
        }

        count += usize::from(matches!(node.kind(), Expr));
        expanded |= matches!(node.kind(), Kind::Comment) || count >= 6;
    }

    o.write(p.expect(open)?)?;

    if expanded {
        exprs_loose(o, p)?;
    } else {
        exprs_compact(o, p)?;
    }

    p.one(close)?.write(o)?;
    Ok(())
}

fn exprs_loose<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.nl(1)?;
    o.indent(1)?;

    while let Some(node) = p.try_pump(Expr)? {
        o.comments(Line)?;
        node.parse(|p| expr(o, p))?;
        p.remaining(o, K![,])?.write(o)?;
        o.nl(1)?;
    }

    o.nl(1)?;
    o.indent(-1)?;
    Ok(())
}

fn exprs_compact<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(Expr)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| expr(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    }

    Ok(())
}

fn expr_binary<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.pump()?.parse(|p| expr(o, p))?;

    while let Some(op) = p.try_pump(ExprOperator)? {
        o.ws()?;
        o.write(op)?;
        o.ws()?;
        p.pump()?.parse(|p| expr(o, p))?;
    }

    Ok(())
}

fn expr_unary<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.pump()?)?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn expr_if<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(If)?)?;
    o.ws()?;
    condition_or_expr(o, p)?;
    o.ws()?;

    if let Some(op) = p.try_pump(Block)? {
        op.parse(|p| block(o, p))?;
    } else {
        o.lit("{}")?;
    }

    for node in p.by_ref() {
        match node.kind() {
            ExprElse => {
                node.parse(|p| {
                    o.ws()?;
                    o.write(p.expect(Else)?)?;
                    o.ws()?;
                    p.expect(Block)?.parse(|p| block(o, p))?;
                    Ok(())
                })?;
            }
            ExprElseIf => {
                node.parse(|p| {
                    o.ws()?;
                    o.write(p.expect(Else)?)?;
                    o.ws()?;
                    o.write(p.expect(If)?)?;
                    o.ws()?;
                    condition_or_expr(o, p)?;
                    o.ws()?;
                    p.expect(Block)?.parse(|p| block(o, p))?;
                    Ok(())
                })?;
            }
            _ => {
                o.write(node)?;
            }
        }
    }

    Ok(())
}

fn expr_while<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![while])?)?;
    o.ws()?;
    condition_or_expr(o, p)?;
    o.ws()?;
    p.expect(Block)?.parse(|p| block(o, p))?;
    Ok(())
}

fn expr_loop<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![loop])?)?;
    o.ws()?;
    p.expect(Block)?.parse(|p| block(o, p))?;
    Ok(())
}

fn expr_for<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![for])?)?;
    o.ws()?;
    p.expect(Pat)?.parse(|p| pat(o, p))?;
    o.ws()?;
    o.write(p.expect(K![in])?)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    o.ws()?;
    p.expect(Block)?.parse(|p| block(o, p))?;
    Ok(())
}

fn expr_break<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![break])?)?;

    if let Some(labels) = p.try_pump(Labels)? {
        for label in labels.into_stream() {
            o.ws()?;
            o.write(label)?;
        }
    }

    if let Some(node) = p.try_pump(Expr)? {
        o.ws()?;
        node.parse(|p| expr(o, p))?;
    }

    Ok(())
}

fn expr_continue<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![continue])?)?;

    if let Some(labels) = p.try_pump(Labels)? {
        for label in labels.into_stream() {
            o.ws()?;
            o.write(label)?;
        }
    }

    Ok(())
}

fn expr_return<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![return])?)?;

    if let Some(node) = p.try_pump(Expr)? {
        o.ws()?;
        node.parse(|p| expr(o, p))?;
    }

    Ok(())
}

fn expr_yield<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![yield])?)?;

    if let Some(node) = p.try_pump(Expr)? {
        o.ws()?;
        node.parse(|p| expr(o, p))?;
    }

    Ok(())
}

fn expr_select<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![select])?)?;
    o.ws()?;

    let Some(open) = p.try_pump(K!['{'])? else {
        o.lit("{}")?;
        return Ok(());
    };

    o.indent(1)?;
    o.write(open)?;

    while matches!(p.peek(), K![default] | Pat) {
        o.nl(1)?;
        o.comments(Line)?;

        match p.peek() {
            K![default] => {
                o.write(p.expect(K![default])?)?;
            }
            _ => {
                p.expect(Pat)?.parse(|p| pat(o, p))?;
            }
        }

        if let Some(eq) = p.try_pump(K![=])? {
            o.ws()?;
            o.write(eq)?;
            o.ws()?;
            p.pump()?.parse(|p| expr(o, p))?;
        }

        o.ws()?;

        p.one(K![=>])?.write(o)?;

        o.ws()?;

        let is_block = p.pump()?.parse(|p| {
            let kind = expr_with_kind(o, p)?;
            Ok(matches!(kind, Block))
        })?;

        p.remaining(o, K![,])?.write_only_if(o, !is_block)?;
    }

    o.comments(Line)?;
    o.nl(1)?;
    o.indent(-1)?;
    p.one(K!['}'])?.write(o)?;
    Ok(())
}

fn expr_match<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![match])?)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    o.ws()?;

    let Some(open) = p.try_pump(K!['{'])? else {
        o.lit("{}")?;
        return Ok(());
    };

    o.write(open)?;
    o.indent(1)?;

    while let Some(node) = p.try_pump(Pat)? {
        o.nl(1)?;
        o.comments(Line)?;

        node.parse(|p| pat(o, p))?;

        if let Some(node) = p.try_pump(K![if])? {
            o.ws()?;
            o.write(node)?;
            o.ws()?;
            p.pump()?.parse(|p| expr(o, p))?;
        }

        o.ws()?;
        p.one(K![=>])?.write(o)?;
        o.ws()?;

        let is_block = p.pump()?.parse(|p| {
            let kind = expr_with_kind(o, p)?;
            Ok(matches!(kind, Block))
        })?;

        p.remaining(o, K![,])?.write_only_if(o, !is_block)?;
    }

    o.comments(Line)?;
    o.nl(1)?;
    o.indent(-1)?;
    p.one(K!['}'])?.write(o)?;
    Ok(())
}

fn expr_closure<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    if let Some(node) = p.try_pump(ClosureArguments)? {
        node.parse(|p| {
            if let Some(open) = p.try_pump(K![||])? {
                o.write(open)?;
            } else {
                o.write(p.expect(K![|])?)?;

                let mut empty = true;
                let mut last_comma = Remaining::default();

                while let Some(node) = p.try_pump(Pat)? {
                    o.comments(Prefix)?;

                    if !empty {
                        last_comma.write(o)?;
                        o.ws()?;
                    }

                    node.parse(|p| pat(o, p))?;
                    last_comma = p.remaining(o, K![,])?;
                    empty = false;
                    o.comments(Suffix)?;
                }

                last_comma.ignore(o)?;

                if empty {
                    o.comments(Infix)?;
                }

                if let Some(node) = p.try_pump(K![|])? {
                    o.write(node)?;
                } else {
                    o.lit("|")?;
                }
            }

            Ok(())
        })?;
    } else {
        o.lit("||")?;
    }

    o.ws()?;

    if let Some(node) = p.try_pump(Expr)? {
        node.parse(|p| expr(o, p))?;
    } else {
        o.lit("{}")?;
    }

    Ok(())
}

fn expr_chain<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let expanded = o.source.is_at_least(p.span(), 80)?;

    // If the first expression *is* small, and there are no other expressions
    // that need indentation in the chain, we can keep it all on one line.
    let head = p.pump()?.parse(|p| {
        let first = p.span();
        expr(o, p)?;
        Ok(first)
    })?;

    let tail = 'tail: {
        for (n, node) in p.children().enumerate() {
            if matches!(node.kind(), ExprCall) {
                break 'tail Some((n, node.span()));
            }
        }

        None
    };

    let first_is_small = if let Some((_, tail)) = tail {
        !o.source.is_at_least(head.join(tail.head()), 80)?
    } else {
        !o.source.is_at_least(head, 80)?
    };

    let from;

    if expanded && first_is_small {
        let mut found = false;
        let first = tail.map(|(n, _)| n).unwrap_or_default();

        for node in p.children().skip(first.wrapping_add(1)) {
            found |= matches!(node.kind(), ExprField | ExprAwait);

            if found {
                break;
            }
        }

        if found {
            from = 0;
        } else {
            from = first + 1;
        }
    } else {
        from = if expanded { 0 } else { usize::MAX };
    }

    let mut unindented = true;

    for (n, node) in p.by_ref().enumerate() {
        if n >= from {
            o.indent(isize::from(take(&mut unindented)))?;
            o.nl(usize::from(matches!(node.kind(), ExprField | ExprAwait)))?;
        }

        node.parse(|p| {
            match p.kind() {
                ExprTry => {
                    p.one(K![?])?.write(o)?;
                }
                ExprAwait => {
                    p.one(K![.])?.write(o)?;
                    p.one(K![await])?.write(o)?;
                }
                ExprField => {
                    p.one(K![.])?.write(o)?;

                    match p.peek() {
                        K![number] => {
                            o.write(p.pump()?)?;
                        }
                        _ => {
                            p.expect(Path)?.parse(|p| path(o, p))?;
                        }
                    }
                }
                ExprCall => {
                    exprs(o, p, K!['('], K![')'])?;
                }
                ExprMacroCall => {
                    o.write(p.expect(K![!])?)?;

                    match p.peek() {
                        K!['{'] => loose_expr_macro_call(o, p)?,
                        _ => compact_expr_macro_call(o, p)?,
                    }
                }
                ExprIndex => {
                    o.write(p.expect(K!['['])?)?;
                    o.comments(Prefix)?;
                    p.pump()?.parse(|p| expr(o, p))?;
                    o.comments(Suffix)?;
                    p.one(K![']'])?.write(o)?;
                }
                _ => {
                    return Err(p.unsupported("expression chain"));
                }
            }

            Ok(())
        })?;
    }

    if !unindented {
        o.indent(-1)?;
    }

    Ok(())
}

fn condition_or_expr<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    if let Some(c) = p.try_pump(Condition)? {
        c.parse(|p| condition(o, p))?;
    } else {
        p.pump()?.parse(|p| expr(o, p))?;
    }

    Ok(())
}

fn condition<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![let])?)?;
    o.ws()?;
    p.expect(Pat)?.parse(|p| pat(o, p))?;
    o.ws()?;
    o.write(p.expect(K![=])?)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn item_struct<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<bool> {
    o.write(p.expect(K![struct])?)?;
    o.ws()?;
    o.write(p.expect(StructName)?)?;

    let body = p.pump()?;

    let needs_semi = match body.kind() {
        StructBody => {
            o.ws()?;
            body.parse(|p| struct_body(o, p))?;
            false
        }
        TupleBody => {
            body.parse(|p| tuple_body(o, p))?;
            true
        }
        EmptyBody => true,
        _ => {
            return Err(body.unsupported("struct declaration"));
        }
    };

    Ok(needs_semi)
}

fn item_enum<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let st = p.expect(K![enum])?;
    o.write(st)?;

    o.ws()?;

    let name = p.expect(EnumName)?;
    o.write(name)?;

    let variants = p.expect(EnumVariants)?;

    variants.parse(|p| {
        o.ws()?;

        let Some(node) = p.try_pump(K!['{'])? else {
            o.lit("{}")?;
            return Ok(());
        };

        o.write(node)?;
        o.indent(1)?;

        let mut empty = true;

        while let Some(node) = p.try_pump(Variant)? {
            o.nl(1)?;
            o.comments(Line)?;
            node.parse(|p| variant(o, p))?;
            empty = false;
        }

        o.comments(Line)?;
        o.nl(usize::from(!empty))?;
        o.indent(-1)?;
        p.one(K!['}'])?.write(o)?;
        Ok(())
    })?;

    Ok(())
}

fn variant<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.comments(Line)?;

    let name = p.expect(VariantName)?;
    o.write(name)?;

    let body = p.pump()?;

    match body.kind() {
        StructBody => {
            o.ws()?;
            body.parse(|p| struct_body(o, p))?;
        }
        TupleBody => {
            body.parse(|p| tuple_body(o, p))?;
        }
        EmptyBody => {}
        _ => {
            return Err(body.unsupported("variant body"));
        }
    }

    p.remaining(o, K![,])?.write(o)?;
    Ok(())
}

fn struct_body<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['{'])?)?;
    o.indent(1)?;
    o.comments(Line)?;

    let mut empty = true;

    while let Some(field) = p.try_pump(Field)? {
        o.nl(1)?;
        o.comments(Line)?;
        field.parse(|p| o.write(p.pump()?))?;
        p.remaining(o, K![,])?.write(o)?;
        empty = false;
    }

    o.comments(Line)?;
    o.nl(usize::from(!empty))?;
    o.indent(-1)?;
    p.one(K!['}'])?.write(o)?;
    Ok(())
}

fn tuple_body<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['('])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(Field)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| o.write(p.pump()?))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    last_comma.ignore(o)?;

    if empty {
        o.comments(Infix)?;
    }

    p.one(K![')'])?.write(o)?;
    o.comments(Suffix)?;
    Ok(())
}

fn item_fn<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![fn])?)?;

    if matches!(p.peek(), K![ident]) {
        o.ws()?;
        o.write(p.pump()?)?;
    }

    if let Some(node) = p.try_pump(FnArgs)? {
        node.parse(|p| fn_args(o, p))?;
    } else {
        o.lit("()")?;
    }

    if let Some(node) = p.try_pump(Block)? {
        o.ws()?;
        node.parse(|p| block(o, p))?;
    } else {
        o.ws()?;
        o.lit("{")?;
        o.nl(1)?;
        o.lit("}")?;
    }

    Ok(())
}

fn item_use<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![use])?)?;
    o.ws()?;
    item_use_path(o, p)
}

fn item_use_path<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    for node in p.by_ref() {
        match node.kind() {
            ItemUseGroup => {
                node.parse(|p: &mut Stream<'a>| item_use_group(o, p))?;
            }
            _ => {
                o.write(node)?;
            }
        }
    }

    Ok(())
}

fn item_use_group<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut nested = 0;

    for n in p.children() {
        nested += usize::from(matches!(n.kind(), ItemUsePath));

        if nested > 1 {
            break;
        }
    }

    let open = p.expect(K!['{'])?;

    if nested == 1 {
        o.ignore(open)?;
    } else {
        o.write(open)?;
    }

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(inner) = p.try_pump(ItemUsePath)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        inner.parse(|p| item_use_path(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    if empty {
        o.comments(Infix)?;
    }

    last_comma.ignore(o)?;

    let close = p.one(K!['}'])?;

    if nested == 1 {
        close.ignore(o)?;
    } else {
        close.write(o)?;
    }

    Ok(())
}

fn item_impl<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K![impl])?)?;
    o.ws()?;
    p.expect(Path)?.parse(|p| path(o, p))?;
    o.ws()?;
    p.expect(Block)?.parse(|p| block(o, p))?;
    Ok(())
}

fn item_mod<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<bool> {
    o.write(p.expect(K![mod])?)?;
    o.ws()?;
    o.write(p.pump()?)?;

    if let Some(node) = p.try_pump(Block)? {
        o.ws()?;
        node.parse(|p| block(o, p))?;
        Ok(false)
    } else {
        Ok(true)
    }
}

fn item_const<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.pump()?)?;
    o.ws()?;
    p.one(K![=])?.write(o)?;
    o.ws()?;
    p.pump()?.parse(|p| expr(o, p))?;
    Ok(())
}

fn fn_args<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    o.write(p.expect(K!['('])?)?;

    let mut empty = true;
    let mut last_comma = Remaining::default();

    while let Some(node) = p.try_pump(Pat)? {
        o.comments(Prefix)?;

        if !empty {
            last_comma.write(o)?;
            o.ws()?;
        }

        node.parse(|p| pat(o, p))?;
        last_comma = p.remaining(o, K![,])?;
        empty = false;
        o.comments(Suffix)?;
    }

    if empty {
        o.comments(Infix)?;
    }

    last_comma.ignore(o)?;
    p.one(K![')'])?.write(o)?;
    Ok(())
}

fn block<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    block_with(o, p, false)
}

fn block_with<'a>(o: &mut Output<'a>, p: &mut Stream<'a>, compact: bool) -> Result<()> {
    p.one(K!['{'])?.write(o)?;

    p.expect(BlockBody)?.parse(|p| {
        let expanded = !p.is_eof() || !compact;

        if expanded {
            o.indent(1)?;
            o.nl(1)?;
            o.comments(Line)?;
        } else {
            o.comments(Prefix)?;
        }

        block_content(o, p)?;

        if expanded {
            o.nl(1)?;
            o.comments(Line)?;
            o.nl(1)?;
            o.indent(-1)?;
        } else {
            o.comments(Suffix)?;
        }

        Ok(())
    })?;

    p.one(K!['}'])?.write(o)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum StmtKind {
    None,
    Const,
    Local,
    Item,
    Expr,
}

/// The contents of a block.
fn block_content<'a>(o: &mut Output<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut last_kind = StmtKind::None;

    while !p.is_eof() {
        let node = p.pump()?;

        let kind = kind_to_stmt_kind(node.kind());

        if !matches!(last_kind, StmtKind::None) {
            let n = match last_kind {
                StmtKind::Item => 1,
                StmtKind::Const => usize::from(!matches!(kind, StmtKind::Const)),
                StmtKind::Local => usize::from(!matches!(kind, StmtKind::Local)),
                _ => 0,
            };

            o.nl(n + 1)?;
        }

        o.comments(Line)?;

        let (kind, needs_semi) = node.parse(|p| stmt(o, p))?;
        let kind = kind_to_stmt_kind(kind);

        let trailing_semi = p.remaining(o, K![;])?;

        if needs_semi || trailing_semi.is_present() {
            o.comments(Suffix)?;
        }

        trailing_semi.write_if(o, needs_semi)?;
        last_kind = kind;
    }

    Ok(())
}

fn kind_to_stmt_kind(kind: Kind) -> StmtKind {
    match kind {
        Local => StmtKind::Local,
        ItemConst => StmtKind::Const,
        ItemStruct => StmtKind::Item,
        ItemEnum => StmtKind::Item,
        ItemFn => StmtKind::Item,
        ItemImpl => StmtKind::Item,
        ItemMod => StmtKind::Item,
        ExprIf => StmtKind::Item,
        ExprFor => StmtKind::Item,
        ExprWhile => StmtKind::Item,
        ExprLoop => StmtKind::Item,
        ExprMatch => StmtKind::Item,
        ExprSelect => StmtKind::Item,
        _ => StmtKind::Expr,
    }
}
