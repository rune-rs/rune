use crate::ast::{Delimiter, Kind};
use crate::grammar::Parser;
use crate::parse::ParseError;

use Kind::*;

type Result<T, E = ParseError> = std::result::Result<T, E>;

pub(crate) fn file(p: &mut Parser<'_>) -> Result<()> {
    p.open(File)?;
    p.eat_matching(|t| matches!(t, K![#!(..)]))?;

    while outer_attribute(p)? {}

    loop {
        let c = p.checkpoint()?;
        let _ = visibility(p)?;

        if !item(p)? {
            break;
        }
    }

    p.close()?;
    Ok(())
}

fn ident(p: &mut Parser<'_>) -> Result<bool> {
    Ok(match p.nth(0)? {
        K![ident] => {
            p.bump()?;
            true
        }
        _ => false,
    })
}

fn visibility(p: &mut Parser<'_>) -> Result<bool> {
    let c = p.checkpoint()?;

    if !p.eat(K![pub])? {
        return Ok(false);
    }

    if p.eat(K!['('])? {
        match p.nth(0)? {
            K![in] => todo!(),
            K![super] => todo!(),
            K![self] => todo!(),
            _ => todo!(),
        }

        if !p.eat(K![')'])? {
            p.bump_until(Whitespace)?;
            p.close_at(c, Error)?;
        }
    }

    p.close_at(c, Visibility)?;
    Ok(true)
}

fn item_fn(p: &mut Parser<'_>) -> Result<()> {
    let c = p.checkpoint()?;
    p.bump()?;
    p.skip()?;

    p.open(ItemFnName)?;
    ident(p)?;
    p.close()?;

    if p.eat(K!['('])? {
        // Eat arguments.

        p.open(ItemFnArguments)?;
        p.close()?;

        if !p.eat(K![')'])? {
            p.close_at(c, Error)?;
        }
    } else {
        p.bump_until(K!['{'])?;
        p.close_at(c, Error)?;
    }

    p.open(ItemFnBody)?;
    block(p)?;
    p.close()?;

    p.close_at(c, ItemFn)?;
    Ok(())
}

fn item(p: &mut Parser<'_>) -> Result<bool> {
    let c = p.checkpoint()?;

    let item = match p.nth(0)? {
        K![fn] => {
            item_fn(p)?;
            true
        }
        _ => false,
    };

    if item {
        p.close_at(c, Item)?;
    }

    Ok(item)
}

fn block(p: &mut Parser<'_>) -> Result<()> {
    let head = p.checkpoint()?;

    if !p.eat(K!['{'])? {
        p.bump_until(K!['{'])?;
        p.close_at(head, Error)?;
        p.bump()?;
    }

    let tail = p.checkpoint()?;

    if !p.eat(K!['}'])? {
        p.bump_until_closed(Delimiter::Brace)?;
        p.close_at(tail, Error)?;
        p.bump()?;
    }

    Ok(())
}

fn outer_attribute(p: &mut Parser<'_>) -> Result<bool> {
    if !matches!((p.nth(0)?, p.nth(1)?), (K![#], K![!])) {
        return Ok(false);
    }

    let c = p.checkpoint()?;

    p.close_at(c, OuterAttribute)?;
    Ok(true)
}
