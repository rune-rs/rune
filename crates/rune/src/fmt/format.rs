use core::mem::take;

use crate::ast::{Delimiter, Kind};
use crate::compile::Result;
use crate::grammar::{classify, object_key, MaybeNode, NodeClass};

use super::{Comments, Formatter, Node, Remaining, Stream, Tree};

use Comments::*;
use Kind::*;

#[derive(Default)]
struct Attrs {
    skip: bool,
}

/// Test if a node is the `#[runefmt::skip]` attribute.
fn is_runefmt_skip<'a>(fmt: &Formatter<'a>, node: Node<'a>) -> bool {
    let mut skip = None;

    _ = node.parse(|p| {
        p.expect(K![#])?;

        p.expect(K!['['])?;

        p.expect(TokenStream)?.parse(|p| {
            let ns = p.pump()?;
            p.expect(K![::])?;
            let name = p.pump()?;

            skip = skip.or(
                match (fmt.source.get(ns.span())?, fmt.source.get(name.span())?) {
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

pub(super) fn root<'a>(fmt: &mut Formatter<'a>, tree: &'a Tree) -> Result<()> {
    tree.parse_all(|p| block_content(fmt, p))?;
    fmt.nl(1)?;
    Ok(())
}

fn expr_labels<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    while matches!(p.peek(), K!['label]) {
        p.pump()?.fmt(fmt)?;
        p.remaining(fmt, K![:])?.fmt(fmt)?;
        fmt.ws()?;
    }

    Ok(())
}

fn inner_attributes<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    while let MaybeNode::Some(attr) = p.eat(InnerAttribute) {
        attr.fmt(fmt)?;
        fmt.nl(1)?;
    }

    Ok(())
}

fn attributes<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    while let MaybeNode::Some(attr) = p.eat(Attribute) {
        attrs.skip |= is_runefmt_skip(fmt, attr.clone());
        attr.fmt(fmt)?;
        fmt.nl(1)?;
    }

    Ok(attrs)
}

fn modifiers<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.eat(Modifiers).parse(|p| {
        let mut any = false;

        for node in p.by_ref() {
            match node.kind() {
                ModifierSuper | ModifierSelf | ModifierCrate => {
                    node.parse(|p| {
                        p.expect(K!['('])?.fmt(fmt)?;
                        fmt.comments(Infix)?;
                        p.pump()?.fmt(fmt)?;
                        fmt.comments(Infix)?;
                        p.one(K![')']).fmt(fmt)?;
                        Ok(())
                    })?;
                }
                ModifierIn => {
                    node.parse(|p| {
                        p.expect(K!['('])?.fmt(fmt)?;
                        p.expect(K![in])?.fmt(fmt)?;
                        fmt.ws()?;
                        p.expect(Path)?.parse(|p| path(fmt, p))?;
                        p.one(K![')']).fmt(fmt)?;
                        Ok(())
                    })?;
                }
                Error => {
                    return Err(p.expected("modifier"));
                }
                _ => {
                    if any {
                        fmt.ws()?;
                    }

                    node.fmt(fmt)?;
                }
            }

            any = true;
        }

        if any {
            fmt.ws()?;
        }

        Ok(())
    })?;

    Ok(())
}

fn item<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let attrs = attributes(fmt, p)?;

    p.pump()?.parse(|p| {
        if attrs.skip {
            p.write_remaining(fmt)?;
            return Ok(());
        }

        match p.kind() {
            ItemStruct => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_struct(fmt, p)?;
                }
            }
            ItemEnum => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_enum(fmt, p)?;
                }
            }
            ItemFn => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_fn(fmt, p)?;
                }
            }
            ItemUse => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_use(fmt, p)?;
                }
            }
            ItemImpl => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_impl(fmt, p)?;
                }
            }
            ItemMod | ItemFileMod => {
                if attrs.skip {
                    p.write_remaining(fmt)?;
                } else {
                    modifiers(fmt, p)?;
                    item_mod(fmt, p)?;
                }
            }
            ItemConst => {
                modifiers(fmt, p)?;
                item_const(fmt, p)?;
            }
            _ => return Err(p.expected(Item)),
        }

        Ok(())
    })
}

fn stmt<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    match p.kind() {
        Local => {
            if attributes(fmt, p)?.skip {
                p.write_remaining(fmt)?;
            } else {
                modifiers(fmt, p)?;
                local(fmt, p)?;
            }
        }
        Item => {
            item(fmt, p)?;
        }
        _ => {
            expr(fmt, p)?;
        }
    }

    Ok(())
}

fn local<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![let])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Pat)?.parse(|p| pat(fmt, p))?;
    fmt.ws()?;
    p.one(K![=]).fmt(fmt)?;
    fmt.ws()?;
    p.expect(Expr)?.parse(|p| expr(fmt, p))?;
    Ok(())
}

fn pat<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut attrs = Attrs::default();

    while let MaybeNode::Some(attr) = p.eat(Attribute) {
        attrs.skip |= is_runefmt_skip(fmt, attr.clone());
        attr.fmt(fmt)?;
        fmt.ws()?;
    }

    p.pump()?.parse(|p| {
        match p.kind() {
            Lit => {
                p.eat(K![-]).fmt(fmt)?;
                p.pump()?.fmt(fmt)?;
            }
            PatIgnore => {
                p.pump()?.fmt(fmt)?;
            }
            K![..] => {
                p.pump()?.fmt(fmt)?;
            }
            Path => {
                path(fmt, p)?;
            }
            PatArray => {
                pat_array(fmt, p)?;
            }
            PatTuple => {
                let trailing = p.eat(Path).parse(|p| path(fmt, p))?.is_none();
                pat_tuple(fmt, p, trailing)?;
            }
            PatObject => {
                pat_object(fmt, p)?;
            }
            _ => {
                return Err(p.expected("pattern"));
            }
        }

        Ok(())
    })
}

fn path<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    for node in p.by_ref() {
        match node.kind() {
            PathGenerics => {
                node.parse(|p| path_generics(fmt, p))?;
            }
            _ => {
                node.fmt(fmt)?;
            }
        }
    }

    Ok(())
}

fn path_generics<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![<])?.fmt(fmt)?;

    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Path) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        node.parse(|p| path(fmt, p))?;
        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    p.one(K![>]).fmt(fmt)?;
    Ok(())
}

fn pat_array<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['['])?.fmt(fmt)?;

    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat_matching(|k| matches!(k, Pat | K![..])) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        match node.kind() {
            K![..] => node.fmt(fmt)?,
            _ => node.parse(|p| pat(fmt, p))?,
        }

        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    p.one(K![']']).fmt(fmt)?;
    Ok(())
}

fn pat_tuple<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>, trailing: bool) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;

    let mut count = 0usize;
    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat_matching(|k| matches!(k, Pat | K![..])) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        match node.kind() {
            K![..] => node.fmt(fmt)?,
            _ => node.parse(|p| pat(fmt, p))?,
        };

        comma = p.remaining(fmt, K![,])?;
        count += 1;
        fmt.comments(Suffix)?;
    }

    if count == 1 && trailing {
        comma.fmt(fmt)?;
    } else {
        comma.ignore(fmt)?;

        if count == 0 {
            fmt.comments(Infix)?;
        }
    }

    p.one(K![')']).fmt(fmt)?;
    Ok(())
}

fn expr_tuple<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;

    let mut count = 0usize;
    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Expr) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        node.parse(|p| expr_discard(fmt, p))?;
        comma = p.remaining(fmt, K![,])?;
        count += 1;
        fmt.comments(Suffix)?;
    }

    if count == 1 {
        comma.fmt(fmt)?;
    } else {
        comma.ignore(fmt)?;

        if count == 0 {
            fmt.comments(Infix)?;
        }
    }

    p.one(K![')']).fmt(fmt)?;
    Ok(())
}

fn expr_object<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            p.expect(AnonymousObjectKey)?.fmt(fmt)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(fmt, p))?;
            fmt.ws()?;
        }
    }

    let mut count = 0;
    let mut expanded = fmt.source.is_at_least(p.span(), 80)?;

    for node in p.children() {
        if expanded {
            break;
        }

        count += usize::from(matches!(node.kind(), object_key!()));
        expanded |= matches!(node.kind(), Kind::Comment) || count >= 6;
    }

    if expanded {
        expr_object_loose(fmt, p)
    } else {
        expr_object_compact(fmt, p)
    }
}

fn expr_object_loose<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['{'])?.fmt(fmt)?;

    fmt.nl(1)?;
    fmt.indent(1)?;

    while matches!(p.peek(), object_key!()) {
        fmt.comments(Line)?;
        p.pump()?.fmt(fmt)?;

        p.eat(K![:]).and_then(|colon| {
            colon.fmt(fmt)?;
            fmt.ws()?;
            p.pump()?.parse(|p| expr(fmt, p))
        })?;

        p.remaining(fmt, K![,])?.fmt(fmt)?;
        fmt.nl(1)?;
    }

    fmt.nl(1)?;
    fmt.indent(-1)?;

    p.remaining(fmt, K!['}'])?.fmt(fmt)?;
    Ok(())
}

fn expr_object_compact<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['{'])?.fmt(fmt)?;

    let mut comma = Remaining::default();

    while matches!(p.peek(), object_key!()) {
        comma.fmt(fmt)?;
        fmt.ws()?;

        p.pump()?.fmt(fmt)?;

        p.eat(K![:]).and_then(|colon| {
            colon.fmt(fmt)?;
            fmt.ws()?;
            p.pump()?.parse(|p| expr(fmt, p))
        })?;

        comma = p.remaining(fmt, K![,])?;
    }

    if comma.ignore(fmt)? {
        fmt.ws()?;
    } else {
        fmt.comments(Infix)?;
    }

    p.remaining(fmt, K!['}'])?.fmt(fmt)?;
    Ok(())
}

fn pat_object<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    match p.peek() {
        AnonymousObjectKey => {
            p.expect(AnonymousObjectKey)?.fmt(fmt)?;
        }
        _ => {
            p.expect(Path)?.parse(|p| path(fmt, p))?;
            fmt.ws()?;
        }
    }

    p.expect(K!['{'])?.fmt(fmt)?;

    let mut comma = Remaining::default();

    while matches!(p.peek(), object_key!() | K![..]) {
        comma.fmt(fmt)?;
        fmt.ws()?;

        match p.peek() {
            object_key!() => {
                p.pump()?.fmt(fmt)?;

                p.eat(K![:]).and_then(|colon| {
                    colon.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(Pat)?.parse(|p| pat(fmt, p))
                })?;
            }
            _ => {
                p.expect(K![..])?.fmt(fmt)?;
            }
        }

        comma = p.remaining(fmt, K![,])?;
    }

    if comma.ignore(fmt)? {
        fmt.ws()?;
    } else {
        fmt.comments(Infix)?;
    }

    p.remaining(fmt, K!['}'])?.fmt(fmt)?;
    Ok(())
}

fn expr_discard<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    expr(fmt, p)?;
    Ok(())
}

fn expr<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<Kind> {
    let mut attrs = Attrs::default();

    while let MaybeNode::Some(attr) = p.eat(Attribute) {
        attrs.skip |= is_runefmt_skip(fmt, attr.clone());
        attr.fmt(fmt)?;
        fmt.ws()?;
    }

    if attrs.skip {
        p.write_remaining(fmt)?;
        Ok(Expr)
    } else {
        modifiers(fmt, p)?;
        expr_labels(fmt, p)?;
        p.pump()?.parse(|p| inner_expr(fmt, p))
    }
}

fn inner_expr<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<Kind> {
    match p.kind() {
        Path => {
            path(fmt, p)?;
        }
        Lit => {
            p.eat(K![-]).fmt(fmt)?;
            p.pump()?.fmt(fmt)?;
        }
        TemplateString => {
            p.pump()?.fmt(fmt)?;
        }
        Block => {
            block_with(fmt, p, true)?;
        }
        ExprAssign => {
            expr_assign(fmt, p)?;
        }
        ExprArray => {
            exprs(fmt, p, K!['['], K![']'])?;
        }
        ExprTuple => {
            expr_tuple(fmt, p)?;
        }
        ExprObject => {
            expr_object(fmt, p)?;
        }
        ExprBinary => {
            expr_binary(fmt, p)?;
        }
        ExprUnary => {
            expr_unary(fmt, p)?;
        }
        ExprGroup => {
            expr_group(fmt, p)?;
        }
        ExprEmptyGroup => {
            expr_empty_group(fmt, p)?;
        }
        ExprIf => {
            expr_if(fmt, p)?;
        }
        ExprWhile => {
            expr_while(fmt, p)?;
        }
        ExprLoop => {
            expr_loop(fmt, p)?;
        }
        ExprBreak => {
            expr_break(fmt, p)?;
        }
        ExprContinue => {
            expr_continue(fmt, p)?;
        }
        ExprReturn => {
            expr_return(fmt, p)?;
        }
        ExprYield => {
            expr_yield(fmt, p)?;
        }
        ExprFor => {
            expr_for(fmt, p)?;
        }
        ExprMatch => {
            expr_match(fmt, p)?;
        }
        ExprSelect => {
            expr_select(fmt, p)?;
        }
        ExprRangeFull => {
            p.pump()?.fmt(fmt)?;
        }
        ExprRangeFrom => {
            p.pump()?.parse(|p| inner_expr(fmt, p))?;
            p.pump()?.fmt(fmt)?;
        }
        ExprRangeTo | ExprRangeToInclusive => {
            p.pump()?.fmt(fmt)?;
            p.pump()?.parse(|p| inner_expr(fmt, p))?;
        }
        ExprRange | ExprRangeInclusive => {
            p.pump()?.parse(|p| inner_expr(fmt, p))?;
            p.pump()?.fmt(fmt)?;
            p.pump()?.parse(|p| inner_expr(fmt, p))?;
        }
        ExprClosure => {
            expr_closure(fmt, p)?;
        }
        ExprChain => {
            expr_chain(fmt, p)?;
        }
        ExprMacroCall => {
            p.expect(Path)?.parse(|p| path(fmt, p))?;
            p.expect(K![!])?.fmt(fmt)?;

            match p.peek() {
                K!['{'] => loose_expr_macro_call(fmt, p)?,
                _ => compact_expr_macro_call(fmt, p)?,
            }
        }
        Error => {
            if fmt.options.error_recovery {
                p.fmt_remaining_trimmed(fmt)?;
            } else {
                return Err(p.expected("inner expression"));
            }
        }
        _ => {
            return Err(p.expected("inner expression"));
        }
    }

    Ok(p.kind())
}

fn loose_expr_macro_call<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['{'])?.fmt(fmt)?;

    p.expect(TokenStream)?.parse(|p| {
        if p.is_eof() {
            return Ok(());
        }

        fmt.nl(1)?;
        fmt.indent(1)?;

        let mut buf = None;
        let mut has_ws = false;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                fmt.write_raw(node)?;
                fmt.nl(1)?;
                has_ws = true;
                continue;
            }

            if node.is_whitespace() {
                buf = Some(node);
                continue;
            }

            if let Some(buf) = buf.take() {
                if !has_ws {
                    fmt.write_raw(buf)?;
                }
            }

            fmt.flush_whitespace(false)?;
            fmt.write_raw(node)?;
            has_ws = false;
        }

        fmt.nl(1)?;
        fmt.indent(-1)?;
        Ok(())
    })?;

    p.expect(K!['}'])?.fmt(fmt)?;
    Ok(())
}

fn compact_expr_macro_call<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;

    p.expect(TokenStream)?.parse(|p| {
        let mut buf = None;
        let mut has_ws = false;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                fmt.write_raw(node)?;
                fmt.ws()?;
                has_ws = true;
                continue;
            }

            if node.is_whitespace() {
                buf = Some(node);
                continue;
            }

            if let Some(buf) = buf.take() {
                if !has_ws {
                    fmt.write_raw(buf)?;
                }
            }

            fmt.flush_whitespace(false)?;
            fmt.write_raw(node)?;
            has_ws = false;
        }

        Ok(())
    })?;

    p.expect(K![')'])?.fmt(fmt)?;
    Ok(())
}

fn expr_assign<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(Expr)?.parse(|p| expr(fmt, p))?;
    fmt.ws()?;
    p.expect(K![=])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Expr)?.parse(|p| expr(fmt, p))?;
    Ok(())
}

fn exprs<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>, open: Kind, close: Kind) -> Result<()> {
    let mut count = 0;
    let mut expanded = fmt.source.is_at_least(p.span(), 80)?;

    for node in p.children() {
        if expanded {
            break;
        }

        count += usize::from(matches!(node.kind(), Expr));
        expanded |= matches!(node.kind(), Kind::Comment) || count >= 6;
    }

    p.one(open).fmt(fmt)?;

    if expanded {
        exprs_loose(fmt, p)?;
    } else {
        exprs_compact(fmt, p)?;
    }

    p.one(close).fmt(fmt)?;
    Ok(())
}

fn exprs_loose<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    fmt.nl(1)?;
    fmt.indent(1)?;

    while let MaybeNode::Some(node) = p.eat(Expr) {
        fmt.comments(Line)?;
        node.parse(|p| expr(fmt, p))?;
        p.remaining(fmt, K![,])?.fmt(fmt)?;
        fmt.nl(1)?;
    }

    fmt.nl(1)?;
    fmt.comments(Line)?;
    fmt.indent(-1)?;
    Ok(())
}

fn exprs_compact<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Expr) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        node.parse(|p| expr(fmt, p))?;
        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    Ok(())
}

fn expr_binary<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.pump()?.parse(|p| inner_expr(fmt, p))?;

    while let MaybeNode::Some(op) = p.eat(ExprOperator) {
        fmt.ws()?;
        op.fmt(fmt)?;
        fmt.ws()?;
        p.pump()?.parse(|p| inner_expr(fmt, p))?;
    }

    Ok(())
}

fn expr_unary<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.pump()?.fmt(fmt)?;
    p.pump()?.parse(|p| inner_expr(fmt, p))?;
    Ok(())
}

fn expr_group<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;

    let mut empty = true;

    p.eat(Expr).parse(|p| {
        fmt.comments(Prefix)?;
        expr(fmt, p)?;
        fmt.comments(Suffix)?;
        empty = false;
        Ok(())
    })?;

    if empty {
        fmt.comments(Infix)?;
    }

    p.one(K![')']).fmt(fmt)?;
    Ok(())
}

fn expr_empty_group<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(Kind::Open(Delimiter::Empty))?.ignore(fmt)?;

    let mut empty = true;

    p.eat(Expr).parse(|p| {
        fmt.comments(Prefix)?;
        expr(fmt, p)?;
        fmt.comments(Suffix)?;
        empty = false;
        Ok(())
    })?;

    if empty {
        fmt.comments(Infix)?;
    }

    p.one(Kind::Open(Delimiter::Empty)).ignore(fmt)?;
    Ok(())
}

fn expr_if<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(If)?.fmt(fmt)?;
    fmt.ws()?;
    condition_or_expr(fmt, p)?;
    fmt.ws()?;

    if p.eat(Block).parse(|p| block(fmt, p))?.is_none() {
        fmt.lit("{}")?;
    }

    for node in p.by_ref() {
        match node.kind() {
            ExprElse => {
                node.parse(|p| {
                    fmt.ws()?;
                    p.expect(K![else])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(Block)?.parse(|p| block(fmt, p))?;
                    Ok(())
                })?;
            }
            ExprElseIf => {
                node.parse(|p| {
                    fmt.ws()?;
                    p.expect(K![else])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(K![if])?.fmt(fmt)?;
                    fmt.ws()?;
                    condition_or_expr(fmt, p)?;
                    fmt.ws()?;
                    p.expect(Block)?.parse(|p| block(fmt, p))?;
                    Ok(())
                })?;
            }
            _ => {
                node.fmt(fmt)?;
            }
        }
    }

    Ok(())
}

fn expr_while<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![while])?.fmt(fmt)?;
    fmt.ws()?;
    condition_or_expr(fmt, p)?;
    fmt.ws()?;
    p.expect(Block)?.parse(|p| block(fmt, p))?;
    Ok(())
}

fn expr_loop<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![loop])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Block)?.parse(|p| block(fmt, p))?;
    Ok(())
}

fn expr_for<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![for])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Pat)?.parse(|p| pat(fmt, p))?;
    fmt.ws()?;
    p.expect(K![in])?.fmt(fmt)?;
    fmt.ws()?;
    p.pump()?.parse(|p| expr(fmt, p))?;
    fmt.ws()?;
    p.expect(Block)?.parse(|p| block(fmt, p))?;
    Ok(())
}

fn expr_break<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![break])?.fmt(fmt)?;

    while matches!(p.peek(), K!['label]) {
        fmt.ws()?;
        p.pump()?.fmt(fmt)?;
    }

    p.eat(Expr).parse(|p| {
        fmt.ws()?;
        expr(fmt, p)
    })?;

    Ok(())
}

fn expr_continue<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![continue])?.fmt(fmt)?;

    while matches!(p.peek(), K!['label]) {
        fmt.ws()?;
        p.pump()?.fmt(fmt)?;
    }

    Ok(())
}

fn expr_return<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![return])?.fmt(fmt)?;

    p.eat(Expr).parse(|p| {
        fmt.ws()?;
        expr(fmt, p)
    })?;

    Ok(())
}

fn expr_yield<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![yield])?.fmt(fmt)?;

    p.eat(Expr).parse(|p| {
        fmt.ws()?;
        expr(fmt, p)
    })?;

    Ok(())
}

fn expr_select<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![select])?.fmt(fmt)?;
    fmt.ws()?;

    let MaybeNode::Some(open) = p.eat(K!['{']) else {
        fmt.lit("{}")?;
        return Ok(());
    };

    fmt.indent(1)?;
    open.fmt(fmt)?;

    while let MaybeNode::Some(node) = p.eat(ExprSelectArm) {
        fmt.nl(1)?;
        fmt.comments(Line)?;

        let is_block = node.parse(|p| {
            match p.peek() {
                K![default] => {
                    p.expect(K![default])?.fmt(fmt)?;
                }
                _ => {
                    p.expect(Pat)?.parse(|p| pat(fmt, p))?;
                }
            }

            p.eat(K![=]).and_then(|eq| {
                fmt.ws()?;
                eq.fmt(fmt)?;
                fmt.ws()?;
                p.pump()?.parse(|p| expr(fmt, p))
            })?;

            fmt.ws()?;
            p.one(K![=>]).fmt(fmt)?;
            fmt.ws()?;

            p.pump()?.parse(|p| {
                let kind = expr(fmt, p)?;
                Ok(matches!(kind, Block))
            })
        })?;

        p.remaining(fmt, K![,])?.write_only_if(fmt, !is_block)?;
    }

    fmt.comments(Line)?;
    fmt.nl(1)?;
    fmt.indent(-1)?;
    p.one(K!['}']).fmt(fmt)?;
    Ok(())
}

fn expr_match<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![match])?.fmt(fmt)?;
    fmt.ws()?;
    p.pump()?.parse(|p| expr(fmt, p))?;
    fmt.ws()?;

    p.one(K!['{']).fmt(fmt)?;

    let mut any = false;

    while let MaybeNode::Some(node) = p.eat(ExprMatchArm) {
        if !any {
            fmt.indent(1)?;
        }

        any = true;

        let is_block = node.parse(|p| {
            fmt.nl(1)?;
            fmt.comments(Line)?;

            p.expect(Pat)?.parse(|p| pat(fmt, p))?;

            p.eat(K![if]).and_then(|node| {
                fmt.ws()?;
                node.fmt(fmt)?;
                fmt.ws()?;
                p.expect(Expr)?.parse(|p| expr(fmt, p))
            })?;

            fmt.ws()?;
            p.one(K![=>]).fmt(fmt)?;
            fmt.ws()?;

            p.pump()?.parse(|p| {
                let kind = expr(fmt, p)?;
                Ok(matches!(kind, Block))
            })
        })?;

        p.remaining(fmt, K![,])?.write_only_if(fmt, !is_block)?;
    }

    if any {
        fmt.comments(Line)?;
        fmt.nl(1)?;
        fmt.indent(-1)?;
    } else {
        fmt.comments(Infix)?;
    }

    p.one(K!['}']).fmt(fmt)?;
    Ok(())
}

fn expr_closure<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    if let MaybeNode::Some(node) = p.eat(ClosureArguments) {
        node.parse(|p| {
            if let MaybeNode::Some(open) = p.eat(K![||]) {
                open.fmt(fmt)?;
                return Ok(());
            }

            p.expect(K![|])?.fmt(fmt)?;

            let mut comma = Remaining::default();

            while let MaybeNode::Some(node) = p.eat(Pat) {
                fmt.comments(Prefix)?;

                if comma.fmt(fmt)? {
                    fmt.ws()?;
                }

                node.parse(|p| pat(fmt, p))?;
                comma = p.remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
            }

            if !comma.ignore(fmt)? {
                fmt.comments(Infix)?;
            }

            p.one(K![|]).fmt(fmt)?;
            Ok(())
        })?;
    } else {
        fmt.lit("||")?;
    }

    fmt.ws()?;

    if p.eat(Expr).parse(|p| expr(fmt, p))?.is_none() {
        fmt.lit("{}")?;
    }

    Ok(())
}

fn expr_chain<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let expanded = fmt.source.is_at_least(p.span(), 80)?;

    // If the first expression *is* small, and there are no other expressions
    // that need indentation in the chain, we can keep it all on one line.
    let head = p.pump()?.parse(|p| {
        let first = p.span();
        inner_expr(fmt, p)?;
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
        !fmt.source.is_at_least(head.join(tail.head()), 80)?
    } else {
        !fmt.source.is_at_least(head, 80)?
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
            fmt.indent(isize::from(take(&mut unindented)))?;
            fmt.nl(usize::from(matches!(node.kind(), ExprField | ExprAwait)))?;
        }

        node.parse(|p| {
            match p.kind() {
                ExprTry => {
                    p.one(K![?]).fmt(fmt)?;
                }
                ExprAwait => {
                    p.one(K![.]).fmt(fmt)?;
                    p.one(K![await]).fmt(fmt)?;
                }
                ExprField => {
                    p.one(K![.]).fmt(fmt)?;

                    match p.peek() {
                        K![number] => {
                            p.pump()?.fmt(fmt)?;
                        }
                        _ => {
                            p.expect(Path)?.parse(|p| path(fmt, p))?;
                        }
                    }
                }
                ExprCall => {
                    exprs(fmt, p, K!['('], K![')'])?;
                }
                ExprIndex => {
                    p.expect(K!['['])?.fmt(fmt)?;
                    fmt.comments(Prefix)?;
                    p.pump()?.parse(|p| expr(fmt, p))?;
                    fmt.comments(Suffix)?;
                    p.one(K![']']).fmt(fmt)?;
                }
                _ => {
                    return Err(p.expected(ExprChain));
                }
            }

            Ok(())
        })?;
    }

    if !unindented {
        fmt.indent(-1)?;
    }

    Ok(())
}

fn condition_or_expr<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    if p.eat(Condition).parse(|p| condition(fmt, p))?.is_none() {
        p.expect(Expr)?.parse(|p| expr(fmt, p))?;
    }

    Ok(())
}

fn condition<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![let])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Pat)?.parse(|p| pat(fmt, p))?;
    fmt.ws()?;
    p.expect(K![=])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Expr)?.parse(|p| expr(fmt, p))?;
    Ok(())
}

fn item_struct<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![struct])?.fmt(fmt)?;

    if matches!(p.peek(), K![ident]) {
        fmt.ws()?;
        p.pump()?.fmt(fmt)?;
    }

    let body = p.pump()?;

    match body.kind() {
        StructBody => {
            fmt.ws()?;
            body.parse(|p| struct_body(fmt, p))?;
        }
        TupleBody => {
            body.parse(|p| tuple_body(fmt, p))?;
        }
        EmptyBody => {}
        _ => {
            return Err(body.unsupported("struct declaration"));
        }
    };

    Ok(())
}

fn item_enum<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![enum])?.fmt(fmt)?;

    if matches!(p.peek(), K![ident]) {
        fmt.ws()?;
        p.pump()?.fmt(fmt)?;
    }

    fmt.ws()?;

    let MaybeNode::Some(node) = p.eat(K!['{']) else {
        fmt.lit("{}")?;
        return Ok(());
    };

    node.fmt(fmt)?;
    fmt.indent(1)?;

    let mut empty = true;

    while let MaybeNode::Some(node) = p.eat(Variant) {
        fmt.nl(1)?;
        fmt.comments(Line)?;
        node.parse(|p| variant(fmt, p))?;
        empty = false;
        p.remaining(fmt, K![,])?.fmt(fmt)?;
    }

    fmt.comments(Line)?;
    fmt.nl(usize::from(!empty))?;
    fmt.indent(-1)?;
    p.one(K!['}']).fmt(fmt)?;
    Ok(())
}

fn variant<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    fmt.comments(Line)?;

    if matches!(p.peek(), K![ident]) {
        p.pump()?.fmt(fmt)?;
    }

    let body = p.pump()?;

    match body.kind() {
        StructBody => {
            fmt.ws()?;
            body.parse(|p| struct_body(fmt, p))?;
        }
        TupleBody => {
            body.parse(|p| tuple_body(fmt, p))?;
        }
        EmptyBody => {}
        _ => {
            return Err(body.unsupported("variant body"));
        }
    }

    Ok(())
}

fn struct_body<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['{'])?.fmt(fmt)?;
    fmt.indent(1)?;
    fmt.comments(Line)?;

    let mut empty = true;

    while let MaybeNode::Some(field) = p.eat(Field) {
        fmt.nl(1)?;
        fmt.comments(Line)?;
        field.parse(|p| p.pump()?.fmt(fmt))?;
        p.remaining(fmt, K![,])?.fmt(fmt)?;
        empty = false;
    }

    fmt.comments(Line)?;
    fmt.nl(usize::from(!empty))?;
    fmt.indent(-1)?;
    p.one(K!['}']).fmt(fmt)?;
    Ok(())
}

fn tuple_body<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;

    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Field) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        node.parse(|p| p.pump()?.fmt(fmt))?;
        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    p.one(K![')']).fmt(fmt)?;
    fmt.comments(Suffix)?;
    Ok(())
}

fn item_fn<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![fn])?.fmt(fmt)?;

    if matches!(p.peek(), K![ident]) {
        fmt.ws()?;
        p.pump()?.fmt(fmt)?;
    }

    if p.eat(FnArgs).parse(|p| fn_args(fmt, p))?.is_none() {
        fmt.lit("()")?;
    }

    fmt.ws()?;

    if p.eat(Block).parse(|p| block(fmt, p))?.is_none() {
        fmt.lit("{")?;
        fmt.nl(1)?;
        fmt.lit("}")?;
    }

    Ok(())
}

fn item_use<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![use])?.fmt(fmt)?;
    fmt.ws()?;
    item_use_path(fmt, p)
}

fn item_use_path<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    while let Some(node) = p.next() {
        match node.kind() {
            ItemUseGroup => {
                node.parse(|p: &mut Stream<'a>| item_use_group(fmt, p))?;
            }
            K![as] => {
                fmt.ws()?;
                node.fmt(fmt)?;

                if let MaybeNode::Some(node) = p.eat_matching(|k| matches!(k, K![ident])) {
                    fmt.ws()?;
                    node.fmt(fmt)?;
                }

                break;
            }
            _ => {
                node.fmt(fmt)?;
            }
        }
    }

    Ok(())
}

fn item_use_group<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut nested = 0;

    for n in p.children() {
        nested += usize::from(matches!(n.kind(), ItemUsePath));

        if nested > 1 {
            break;
        }
    }

    let open = p.expect(K!['{'])?;

    if nested == 1 {
        fmt.ignore(open)?;
    } else {
        open.fmt(fmt)?;
    }

    let mut comma = Remaining::default();

    while let MaybeNode::Some(inner) = p.eat(ItemUsePath) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        inner.parse(|p| item_use_path(fmt, p))?;
        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    let close = p.one(K!['}']);

    if nested == 1 {
        close.ignore(fmt)?;
    } else {
        close.fmt(fmt)?;
    }

    Ok(())
}

fn item_impl<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![impl])?.fmt(fmt)?;
    fmt.ws()?;
    p.expect(Path)?.parse(|p| path(fmt, p))?;
    fmt.ws()?;
    p.expect(Block)?.parse(|p| block(fmt, p))?;
    Ok(())
}

fn item_mod<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![mod])?.fmt(fmt)?;
    fmt.ws()?;
    p.pump()?.fmt(fmt)?;

    p.eat(Block).parse(|p| {
        fmt.ws()?;
        block(fmt, p)
    })?;

    Ok(())
}

fn item_const<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.pump()?.fmt(fmt)?;
    fmt.ws()?;
    p.one(K![=]).fmt(fmt)?;
    fmt.ws()?;
    p.pump()?.parse(|p| expr(fmt, p))?;
    Ok(())
}

fn fn_args<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K!['('])?.fmt(fmt)?;
    p.remaining(fmt, K![,])?.ignore(fmt)?;

    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Pat) {
        fmt.comments(Prefix)?;

        if comma.fmt(fmt)? {
            fmt.ws()?;
        }

        node.parse(|p| pat(fmt, p))?;
        comma = p.remaining(fmt, K![,])?;
        fmt.comments(Suffix)?;
    }

    if !comma.ignore(fmt)? {
        fmt.comments(Infix)?;
    }

    p.one(K![')']).fmt(fmt)?;
    Ok(())
}

fn block<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    block_with(fmt, p, false)
}

fn block_with<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>, compact: bool) -> Result<()> {
    p.one(K!['{']).fmt(fmt)?;

    p.expect(BlockBody)?.parse(|p| {
        let expanded = !p.is_eof() || !compact;

        if expanded {
            fmt.indent(1)?;
            fmt.nl(1)?;
            fmt.comments(Line)?;
        } else {
            fmt.comments(Prefix)?;
        }

        block_content(fmt, p)?;

        if expanded {
            fmt.nl(1)?;
            fmt.comments(Line)?;
            fmt.nl(1)?;
            fmt.indent(-1)?;
        } else {
            fmt.comments(Suffix)?;
        }

        Ok(())
    })?;

    p.one(K!['}']).fmt(fmt)?;
    Ok(())
}

/// The contents of a block.
fn block_content<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    inner_attributes(fmt, p)?;

    let mut last_kind = None;

    while !p.is_eof() {
        let node = p.pump()?;
        let (needs_semi, class) = classify(&node);

        if let Some(last_kind) = last_kind {
            let n = match last_kind {
                NodeClass::Item => 1,
                NodeClass::Const => usize::from(!matches!(class, NodeClass::Const)),
                NodeClass::Local => usize::from(!matches!(class, NodeClass::Local)),
                _ => 0,
            };

            fmt.nl(n + 1)?;
        }

        fmt.comments(Line)?;

        node.parse(|p| stmt(fmt, p))?;

        let trailing_semi = p.remaining(fmt, K![;])?;

        if needs_semi || trailing_semi.is_present() {
            fmt.comments(Suffix)?;
        }

        trailing_semi.write_if(fmt, needs_semi)?;
        last_kind = Some(class);
    }

    Ok(())
}
