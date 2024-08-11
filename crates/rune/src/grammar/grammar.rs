use crate::ast::{self, Kind};
use crate::compile::{ErrorKind, Result};

use super::{Checkpoint, Parser};

use Kind::*;

trait ExprCx {
    fn recover(&self, p: &mut Parser<'_>) -> Result<()>;
}

/// Simple context which performs no recovery.
struct ErrorCx;

impl ExprCx for ErrorCx {
    fn recover(&self, p: &mut Parser<'_>) -> Result<()> {
        Err(p.unsupported(0, "expression")?)
    }
}

#[derive(Default)]
struct Modifiers {
    is_pub: bool,
    is_const: bool,
    is_async: bool,
}

#[derive(Debug, Clone, Copy)]
enum Brace {
    Yes,
    No,
}

enum Binary {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy)]
enum Range {
    Yes,
    No,
}

enum InExpr {
    Yes,
    No,
}

macro_rules! __ws {
    () => {
        $crate::ast::Kind::Whitespace
            | $crate::ast::Kind::Comment
            | $crate::ast::Kind::MultilineComment(..)
    };
}

macro_rules! __object_key {
    () => {
        K![ident] | K![str]
    };
}

pub(crate) use __object_key as object_key;
pub(crate) use __ws as ws;

macro_rules! lit {
    () => {
        K![true] | K![false] | K![number] | K![str] | K![bytestr] | K![char] | K![byte]
    };
}

macro_rules! path_component {
    () => {
        K![::] | K![ident] | K![super] | K![crate] | K![self] | K![Self]
    }
}

pub(super) fn root(p: &mut Parser<'_>) -> Result<()> {
    p.open(Root)?;

    while !p.is_eof()? {
        stmt(p)?;
    }

    p.close()?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn stmt(p: &mut Parser<'_>) -> Result<()> {
    struct StmtCx;

    #[tracing::instrument(skip(p))]
    fn is_stmt_recovering(p: &mut Parser<'_>) -> Result<bool> {
        let is = match p.peek()? {
            Eof => false,
            K![;] => false,
            K![let] => false,
            K![use] => false,
            K![struct] => false,
            K![enum] => false,
            K![fn] => false,
            K![impl] => false,
            K![mod] => false,
            K![ident] => false,
            K![pub] => false,
            K![const] => false,
            K![async] => false,
            K![#] => !matches!(p.nth(1)?, K!['['] | K![!]),
            _ => true,
        };

        Ok(is)
    }

    impl ExprCx for StmtCx {
        fn recover(&self, p: &mut Parser<'_>) -> Result<()> {
            while is_stmt_recovering(p)? {
                p.bump()?;
            }

            Ok(())
        }
    }

    let c = p.checkpoint()?;
    attributes(p)?;
    let m = modifiers(p)?;

    let cx = StmtCx;

    let kind = match p.peek()? {
        K![let] => {
            local(p, &cx)?;
            Local
        }
        K![use] => {
            item_use(p)?;
            ItemUse
        }
        K![struct] => {
            item_struct(p)?;
            ItemStruct
        }
        K![enum] => {
            item_enum(p)?;
            ItemEnum
        }
        K![fn] => {
            item_fn(p)?;
            ItemFn
        }
        K![impl] => {
            item_impl(p)?;
            ItemImpl
        }
        K![mod] => {
            item_mod(p)?;
            ItemMod
        }
        K![ident] if m.is_const => {
            item_const(p)?;
            ItemConst
        }
        _ => {
            labels(p)?;

            if matches!(
                outer_expr_with(p, Brace::Yes, Range::Yes, Binary::Yes, &cx)?,
                Error
            ) {
                Error
            } else {
                Expr
            }
        }
    };

    p.close_at(&c, kind)?;
    p.bump_while(K![;])?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_const(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;
    p.bump_if(K![=])?;
    expr(p)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn attributes(p: &mut Parser<'_>) -> Result<()> {
    while matches!((p.peek()?, p.glued(1)?), (K![#], K![!]) | (K![#], K!['['])) {
        let c = p.checkpoint()?;

        p.bump()?;
        p.bump_if(K![!])?;

        if p.bump_if(K!['['])? {
            token_stream(p, brackets)?;
            p.bump()?;
        }

        p.close_at(&c, Attribute)?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn modifiers(p: &mut Parser<'_>) -> Result<Modifiers> {
    let c = p.checkpoint()?;

    let mut mods = Modifiers::default();
    let mut has_mods = false;

    loop {
        match p.peek()? {
            K![pub] => {
                mods.is_pub = true;
                p.bump()?;

                if p.peek()? == K!['('] {
                    let c = p.checkpoint()?;
                    p.bump()?;

                    let kind = match p.peek()? {
                        K![crate] => {
                            p.bump()?;
                            ModifierCrate
                        }
                        K![in] => {
                            p.bump()?;
                            path(p, InExpr::Yes)?;
                            ModifierIn
                        }
                        _ => Error,
                    };

                    p.bump_if(K![')'])?;
                    p.close_at(&c, kind)?;
                }
            }
            K![const] => {
                mods.is_const = true;
                p.bump()?;
            }
            K![async] => {
                mods.is_async = true;
                p.bump()?;
            }
            _ => {
                break;
            }
        }

        has_mods = true;
    }

    if has_mods {
        p.close_at(&c, Modifiers)?;
    }

    Ok(mods)
}

#[tracing::instrument(skip_all)]
fn local(p: &mut Parser<'_>, cx: &dyn ExprCx) -> Result<()> {
    p.bump()?;
    pat(p)?;
    p.bump_if(K![=])?;
    expr_with(p, Brace::Yes, Range::Yes, Binary::Yes, cx)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_struct(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K![ident]) {
        p.bump()?;
    }

    match p.peek()? {
        K!['{'] => {
            struct_body(p)?;
        }
        K!['('] => {
            tuple_body(p)?;
        }
        _ => {
            empty_body(p)?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_use(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    loop {
        match p.peek()? {
            path_component!() | K![*] => {
                p.bump()?;
            }
            K!['{'] => {
                let c = p.checkpoint()?;
                p.bump()?;

                while matches!(p.peek()?, path_component!() | K!['{'] | K![*]) {
                    let c = p.checkpoint()?;
                    item_use(p)?;
                    p.close_at(&c, ItemUsePath)?;
                    p.bump_while(K![,])?;
                }

                p.bump_if(K!['}'])?;
                p.close_at(&c, ItemUseGroup)?;
            }
            _ => break,
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_enum(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K![ident]) {
        p.bump()?;
    }

    if p.bump_if(K!['{'])? {
        while matches!(p.peek()?, K![ident]) {
            let variant = p.checkpoint()?;

            p.bump()?;

            match p.peek()? {
                K!['{'] => {
                    struct_body(p)?;
                }
                K!['('] => {
                    tuple_body(p)?;
                }
                _ => {
                    p.empty(EmptyBody)?;
                }
            }

            p.bump_while(K![,])?;
            p.close_at(&variant, Variant)?;
        }

        p.bump_if(K!['}'])?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn empty_body(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;
    p.close_at(&c, EmptyBody)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn struct_body(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;

    p.bump()?;

    while matches!(p.peek()?, K![ident]) {
        let c = p.checkpoint()?;
        p.bump()?;
        p.close_at(&c, Field)?;
        p.bump_while(K![,])?;
    }

    p.bump_if(K!['}'])?;
    p.close_at(&c, StructBody)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn tuple_body(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;

    p.bump()?;

    while matches!(p.peek()?, K![ident]) {
        let c = p.checkpoint()?;
        p.bump()?;
        p.close_at(&c, Field)?;
        p.bump_while(K![,])?;
    }

    p.bump_if(K![')'])?;
    p.close_at(&c, TupleBody)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_fn(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K![ident]) {
        p.bump()?;
    }

    if p.peek()? == K!['('] {
        let c = p.checkpoint()?;
        p.bump()?;

        while is_pat(p)? {
            pat(p)?;
            p.bump_while(K![,])?;
        }

        p.bump_if(K![')'])?;
        p.close_at(&c, FnArgs)?;
    }

    if p.peek()? == K!['{'] {
        block(p)?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_impl(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, path_component!()) {
        path(p, InExpr::No)?;
    }

    block(p)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn item_mod(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K![ident]) {
        p.bump()?;
    }

    if matches!(p.peek()?, K!['{']) {
        block(p)?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
#[allow(unused)]
fn is_pat(p: &mut Parser<'_>) -> Result<bool> {
    Ok(match p.peek()? {
        path_component!() => true,
        lit!() => true,
        K![_] => true,
        K![..] => true,
        K!['('] => true,
        K!['['] => true,
        K![#] => matches!(p.glued(1)?, K!['{']),
        _ => false,
    })
}

#[tracing::instrument(skip_all)]
fn pat(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;
    attributes(p)?;

    match p.peek()? {
        lit!() => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, PatLit)?;
        }
        K![_] => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, PatIgnore)?;
        }
        K![..] => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, PatRest)?;
        }
        path_component!() => {
            let c = p.checkpoint()?;
            path(p, InExpr::No)?;

            match p.peek()? {
                K!['{'] => {
                    pat_object(p)?;
                    p.close_at(&c, PatObject)?;
                }
                K!['('] => {
                    parenthesized(p, is_pat, pat, K![')'])?;
                    p.close_at(&c, PatTuple)?;
                }
                _ => {}
            }
        }
        K!['['] => {
            let c = p.checkpoint()?;
            parenthesized(p, is_pat, pat, K![']'])?;
            p.close_at(&c, PatArray)?;
        }
        K!['('] => {
            let c = p.checkpoint()?;
            parenthesized(p, is_pat, pat, K![')'])?;
            p.close_at(&c, PatTuple)?;
        }
        K![#] if matches!(p.glued(1)?, K!['{']) => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, AnonymousObjectKey)?;
            pat_object(p)?;
            p.close_at(&c, PatObject)?;
        }
        _ => {
            let c = p.checkpoint()?;
            p.close_at(&c, Error)?;
        }
    }

    p.close_at(&c, Pat)?;
    Ok(())
}

fn is_expr(p: &mut Parser<'_>) -> Result<bool> {
    is_expr_with(p, Brace::Yes, Range::Yes)
}

#[tracing::instrument(skip(p))]
fn is_expr_with(p: &mut Parser<'_>, brace: Brace, range: Range) -> Result<bool> {
    Ok(match p.peek()? {
        path_component!() => true,
        K![async] => true,
        K![break] => true,
        K![continue] => true,
        K![for] => true,
        K![if] => true,
        K![let] => true,
        K![loop] => true,
        K![match] => true,
        K![return] => true,
        K![select] => true,
        K![while] => true,
        lit!() => true,
        K![||] | K![|] => true,
        K![#] => true,
        K![-] => true,
        K![!] => true,
        K![&] => true,
        K![*] => true,
        K!['label] => matches!(p.glued(1)?, K![:]),
        K!['('] => true,
        K!['['] => true,
        K!['{'] => matches!(brace, Brace::Yes),
        K![..] => matches!(range, Range::Yes),
        K![..=] => matches!(range, Range::Yes),
        TemplateString => true,
        _ => false,
    })
}

#[tracing::instrument(skip_all)]
fn expr(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;
    attributes(p)?;
    modifiers(p)?;
    labels(p)?;
    let cx = ErrorCx;
    outer_expr_with(p, Brace::Yes, Range::Yes, Binary::Yes, &cx)?;
    p.close_at(&c, Expr)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_with(
    p: &mut Parser<'_>,
    brace: Brace,
    range: Range,
    binary: Binary,
    cx: &dyn ExprCx,
) -> Result<()> {
    let c = p.checkpoint()?;
    attributes(p)?;
    modifiers(p)?;
    labels(p)?;
    outer_expr_with(p, brace, range, binary, cx)?;
    p.close_at(&c, Expr)?;
    Ok(())
}

fn is_range(kind: Kind) -> bool {
    match kind {
        ExprRangeTo => true,
        ExprRangeToInclusive => true,
        ExprRangeFull => true,
        ExprRange => true,
        ExprRangeInclusive => true,
        ExprRangeFrom => true,
        _ => false,
    }
}

fn outer_expr_with(
    p: &mut Parser<'_>,
    brace: Brace,
    range: Range,
    binary: Binary,
    cx: &dyn ExprCx,
) -> Result<Kind> {
    let c = p.checkpoint()?;
    let mut kind = expr_primary(p, brace, range, cx)?;

    if is_range(kind) {
        return Ok(kind);
    }

    kind = expr_chain(p, &c, kind)?;

    if p.peek()? == K![=] {
        p.bump()?;
        expr_with(p, brace, Range::Yes, Binary::Yes, cx)?;
        p.close_at(&c, ExprAssign)?;
        return Ok(ExprAssign);
    }

    if matches!(binary, Binary::Yes) {
        let lookahead = ast::BinOp::from_peeker(p)?;

        kind = if expr_binary(p, lookahead, 0, brace, cx)? {
            p.close_at(&c, ExprBinary)?;
            ExprBinary
        } else {
            kind
        };
    }

    if matches!(range, Range::Yes) {
        kind = expr_range(p, &c, kind, brace)?;
    }

    Ok(kind)
}

fn labels(p: &mut Parser<'_>) -> Result<()> {
    while matches!(p.peek()?, K!['label]) {
        p.bump()?;
        p.bump_while(K![:])?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_primary(p: &mut Parser<'_>, brace: Brace, range: Range, cx: &dyn ExprCx) -> Result<Kind> {
    let c = p.checkpoint()?;

    let kind = match p.peek()? {
        lit!() => {
            p.bump()?;
            ExprLit
        }
        path_component!() => {
            path(p, InExpr::Yes)?;

            match p.peek()? {
                K!['{'] if matches!(brace, Brace::Yes) => {
                    expr_object(p)?;
                    ExprObject
                }
                K![!] if matches!(p.glued(1)?, K!['(']) => {
                    p.bump()?;
                    p.bump()?;
                    token_stream(p, parens)?;
                    p.bump()?;
                    ExprMacroCall
                }
                K![!] if matches!(p.glued(1)?, K!['{']) => {
                    p.bump()?;
                    p.bump()?;
                    token_stream(p, braces)?;
                    p.bump()?;
                    ExprMacroCall
                }
                _ => ExprPath,
            }
        }
        K!['('] => expr_tuple_or_group(p)?,
        K!['['] => {
            parenthesized(p, is_expr, expr, K![']'])?;
            ExprArray
        }
        K!['{'] => {
            block_with(p)?;
            Block
        }
        TemplateString => {
            p.bump()?;
            TemplateString
        }
        K![||] => {
            p.push(ClosureArguments)?;
            expr_with(p, brace, range, Binary::Yes, cx)?;
            ExprClosure
        }
        K![|] => {
            let args = p.checkpoint()?;
            parenthesized(p, is_pat, pat, K![|])?;
            p.close_at(&args, ClosureArguments)?;

            expr_with(p, brace, range, Binary::Yes, cx)?;
            ExprClosure
        }
        K![!] | K![-] | K![&] | K![*] => {
            p.bump()?;
            expr_with(p, brace, range, Binary::No, cx)?;
            ExprUnary
        }
        K![if] => {
            expr_if(p)?;
            ExprIf
        }
        K![while] => {
            expr_while(p)?;
            ExprWhile
        }
        K![loop] => {
            expr_loop(p)?;
            ExprLoop
        }
        K![break] => {
            p.bump()?;

            labels(p)?;

            if is_expr_with(p, brace, range)? {
                let cx = ErrorCx;
                expr_with(p, brace, range, Binary::Yes, &cx)?;
            }

            ExprBreak
        }
        K![continue] => {
            p.bump()?;
            labels(p)?;
            ExprContinue
        }
        K![return] => {
            p.bump()?;

            if is_expr_with(p, brace, range)? {
                let cx = ErrorCx;
                expr_with(p, brace, range, Binary::Yes, &cx)?;
            }

            ExprReturn
        }
        K![yield] => {
            p.bump()?;

            if is_expr_with(p, brace, range)? {
                let cx = ErrorCx;
                expr_with(p, brace, range, Binary::Yes, &cx)?;
            }

            ExprYield
        }
        K![for] => {
            expr_for(p)?;
            ExprFor
        }
        K![select] => {
            expr_select(p)?;
            ExprSelect
        }
        K![match] => {
            expr_match(p)?;
            ExprMatch
        }
        K![..] if matches!(range, Range::Yes) => {
            p.bump()?;

            if is_expr_with(p, brace, Range::No)? {
                let cx = ErrorCx;
                outer_expr_with(p, brace, Range::No, Binary::Yes, &cx)?;
                ExprRangeTo
            } else {
                ExprRangeFull
            }
        }
        K![..=] if matches!(range, Range::Yes) => {
            p.bump()?;
            let cx = ErrorCx;
            outer_expr_with(p, brace, Range::No, Binary::Yes, &cx)?;
            ExprRangeToInclusive
        }
        K![#] if matches!(p.glued(1)?, K!['{']) => {
            let key = p.checkpoint()?;
            p.bump()?;
            p.close_at(&key, AnonymousObjectKey)?;
            expr_object(p)?;
            ExprObject
        }
        _ => {
            cx.recover(p)?;
            Error
        }
    };

    p.close_at(&c, kind)?;
    Ok(kind)
}

fn kind_is_callable(kind: Kind) -> bool {
    match kind {
        ExprWhile => false,
        ExprLoop => true,
        ExprFor => false,
        ExprIf => true,
        ExprMatch => true,
        ExprSelect => true,
        _ => true,
    }
}

#[tracing::instrument(skip_all)]
fn expr_chain(p: &mut Parser<'_>, c: &Checkpoint, mut kind: Kind) -> Result<Kind> {
    let mut before = p.checkpoint()?;
    let mut has_chain = false;

    while !p.is_eof()? {
        let is_callable = kind_is_callable(kind);

        let k = match p.peek()? {
            K!['['] if is_callable => {
                p.bump()?;
                expr(p)?;
                p.bump_if(K![']'])?;
                ExprIndex
            }
            // Chained function call.
            K!['('] if is_callable => {
                parenthesized(p, is_expr, expr, K![')'])?;
                ExprCall
            }
            K![?] => {
                p.bump()?;
                ExprTry
            }
            K![.] => {
                p.bump()?;

                match p.peek()? {
                    // <expr>.await
                    K![await] => {
                        p.bump()?;
                        ExprAwait
                    }
                    // <expr>.field
                    path_component!() => {
                        path(p, InExpr::No)?;
                        ExprField
                    }
                    // <expr>.<number>
                    K![number] => {
                        p.bump()?;
                        ExprField
                    }
                    _ => Error,
                }
            }
            _ => {
                break;
            }
        };

        p.close_at(&before, k)?;
        kind = k;
        before = p.checkpoint()?;
        has_chain = true;
    }

    if has_chain {
        p.close_at(c, ExprChain)?;
    }

    Ok(kind)
}

#[tracing::instrument(skip_all)]
fn expr_tuple_or_group(p: &mut Parser<'_>) -> Result<Kind> {
    p.bump()?;

    let mut is_tuple = false;

    while is_expr(p)? {
        expr(p)?;

        if p.bump_while(K![,])? {
            is_tuple = true;
        }
    }

    p.bump_if(K![')'])?;
    let kind = if is_tuple { ExprTuple } else { ExprGroup };
    Ok(kind)
}

#[tracing::instrument(skip_all)]
fn expr_object(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    while matches!(p.peek()?, object_key!()) {
        p.bump()?;

        if p.bump_if(K![:])? {
            expr(p)?;
        }

        p.bump_while(K![,])?;
    }

    p.bump_if(K!['}'])?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn pat_object(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    loop {
        match p.peek()? {
            object_key!() => {
                p.bump()?;

                if p.bump_if(K![:])? {
                    pat(p)?;
                }

                p.bump_while(K![,])?;
            }
            K![..] => {
                p.bump()?;
            }
            _ => {
                break;
            }
        }
    }

    p.bump_if(K!['}'])?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_if(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;
    condition(p)?;
    block(p)?;

    while p.peek()? == K![else] {
        let c = p.checkpoint()?;
        p.bump()?;

        let else_if = if p.bump_if(K![if])? {
            condition(p)?;
            true
        } else {
            false
        };

        block(p)?;

        if else_if {
            p.close_at(&c, ExprElseIf)?;
        } else {
            p.close_at(&c, ExprElse)?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_while(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;
    condition(p)?;
    block(p)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_loop(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;
    block(p)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_for(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;
    pat(p)?;

    if p.bump_if(K![in])? {
        let cx = ErrorCx;
        expr_with(p, Brace::No, Range::Yes, Binary::Yes, &cx)?;
    }

    block(p)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_match(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    let cx = ErrorCx;
    expr_with(p, Brace::No, Range::Yes, Binary::Yes, &cx)?;

    if matches!(p.peek()?, K!['{']) {
        p.bump()?;

        while is_pat(p)? {
            pat(p)?;

            if p.bump_if(K![if])? {
                expr(p)?;
            }

            if p.bump_if(K![=>])? {
                expr(p)?;
            }

            p.bump_while(K![,])?;
        }

        p.bump_if(K!['}'])?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_select(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K!['{']) {
        p.bump()?;

        while is_pat(p)? || matches!(p.peek()?, K![default]) {
            match p.peek()? {
                K![default] => {
                    p.bump()?;
                }
                _ => {
                    pat(p)?;

                    if p.bump_if(K![=])? {
                        expr(p)?;
                    }
                }
            }

            if p.bump_if(K![=>])? {
                expr(p)?;
            }

            p.bump_while(K![,])?;
        }

        p.bump_if(K!['}'])?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn condition(p: &mut Parser<'_>) -> Result<()> {
    if p.peek()? == K![let] {
        let c = p.checkpoint()?;
        p.bump()?;
        pat(p)?;

        if p.peek()? == K![=] {
            p.bump()?;
            let cx = ErrorCx;
            expr_with(p, Brace::No, Range::Yes, Binary::Yes, &cx)?;
        }

        p.close_at(&c, Condition)?;
    } else {
        let cx = ErrorCx;
        expr_with(p, Brace::No, Range::Yes, Binary::Yes, &cx)?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn path(p: &mut Parser<'_>, in_expr: InExpr) -> Result<()> {
    let c = p.checkpoint()?;

    p.bump()?;

    loop {
        // We can't parse generics in expressions, since they would be ambiguous
        // with expressions such as `self::FOO< 10`.
        if matches!(in_expr, InExpr::No) && matches!(p.glued(0)?, K![<]) {
            let c = p.checkpoint()?;
            p.bump()?;

            while matches!(p.peek()?, path_component!()) {
                path(p, InExpr::No)?;
                p.bump_while(K![,])?;
            }

            p.bump_if(K![>])?;
            p.close_at(&c, PathGenerics)?;
        }

        if !matches!(p.peek()?, path_component!()) {
            break;
        }

        p.bump()?;
    }

    p.close_at(&c, Path)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn expr_binary(
    p: &mut Parser<'_>,
    mut lookahead: Option<ast::BinOp>,
    min_precedence: usize,
    brace: Brace,
    cx: &dyn ExprCx,
) -> Result<bool> {
    let mut has_any = false;

    while let Some(op) = lookahead.take() {
        if matches!(op, ast::BinOp::DotDot(..) | ast::BinOp::DotDotEq(..)) {
            break;
        }

        let precedence = op.precedence();

        if precedence < min_precedence {
            break;
        }

        let op_c = p.checkpoint()?;
        op.advance(p)?;
        p.close_at(&op_c, ExprOperator)?;

        let c = p.checkpoint()?;

        expr_with(p, brace, Range::No, Binary::No, cx)?;
        has_any = true;

        lookahead = ast::BinOp::from_peeker(p)?;

        if matches!(
            lookahead,
            Some(ast::BinOp::DotDot(..) | ast::BinOp::DotDotEq(..))
        ) {
            break;
        }

        while let Some(next) = lookahead {
            match (precedence, next.precedence()) {
                (lh, rh) if lh < rh => {
                    // Higher precedence elements require us to recurse.
                    if expr_binary(p, Some(next), lh + 1, brace, cx)? {
                        p.close_at(&c, ExprBinary)?;
                    }

                    lookahead = ast::BinOp::from_peeker(p)?;
                    continue;
                }
                (lh, rh) if lh == rh => {
                    if !next.is_assoc() {
                        return Err(p.error(c.span(), ErrorKind::PrecedenceGroupRequired)?);
                    }
                }
                _ => {}
            };

            break;
        }
    }

    Ok(has_any)
}

#[tracing::instrument(skip_all)]
fn expr_range(p: &mut Parser<'_>, c: &Checkpoint, kind: Kind, brace: Brace) -> Result<Kind> {
    let kind = match p.peek()? {
        K![..] => {
            p.bump()?;

            if is_expr_with(p, brace, Range::No)? {
                let cx = ErrorCx;
                outer_expr_with(p, brace, Range::No, Binary::Yes, &cx)?;
                ExprRange
            } else {
                ExprRangeFrom
            }
        }
        K![..=] => {
            p.bump()?;

            if is_expr_with(p, brace, Range::No)? {
                let cx = ErrorCx;
                outer_expr_with(p, brace, Range::No, Binary::Yes, &cx)?;
                ExprRangeInclusive
            } else {
                Error
            }
        }
        _ => {
            return Ok(kind);
        }
    };

    p.close_at(c, kind)?;
    Ok(kind)
}
#[tracing::instrument(skip_all)]
fn block(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;
    block_with(p)?;
    p.close_at(&c, Block)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn block_with(p: &mut Parser<'_>) -> Result<()> {
    if p.bump_if(K!['{'])? {
        let c = p.checkpoint()?;

        while !matches!(p.peek()?, K!['}'] | Eof) {
            stmt(p)?;
        }

        p.close_at(&c, BlockBody)?;
    }

    p.bump_if(K!['}'])?;
    Ok(())
}

#[tracing::instrument(skip(p, is, parser))]
fn parenthesized(
    p: &mut Parser,
    is: fn(&mut Parser<'_>) -> Result<bool>,
    parser: fn(&mut Parser<'_>) -> Result<()>,
    end: Kind,
) -> Result<()> {
    p.bump()?;

    while is(p)? {
        parser(p)?;
        p.bump_while(K![,])?;
    }

    p.bump_if(end)?;
    Ok(())
}

fn brackets(kind: Kind) -> i32 {
    match kind {
        K!['['] => 1,
        K![']'] => -1,
        _ => 0,
    }
}

fn parens(kind: Kind) -> i32 {
    match kind {
        K!['('] => 1,
        K![')'] => -1,
        _ => 0,
    }
}

fn braces(kind: Kind) -> i32 {
    match kind {
        K!['{'] => 1,
        K!['}'] => -1,
        _ => 0,
    }
}

/// Consumes a token stream out of balanced brackets.
#[tracing::instrument(skip_all)]
fn token_stream(p: &mut Parser<'_>, matcher: fn(Kind) -> i32) -> Result<()> {
    let c = p.checkpoint()?;

    let mut level = 1u32;

    while level > 0 {
        let m = matcher(p.peek()?);

        level = level.wrapping_add_signed(m);

        if level == 0 {
            break;
        }

        p.bump()?;
    }

    p.close_at(&c, TokenStream)?;
    Ok(())
}
