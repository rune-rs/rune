use crate::alloc::Vec;
use crate::ast::{self, Delimiter, Kind, Span};
use crate::compile::{Error, ErrorKind, Result, WithSpan};

use super::{Checkpoint, Parser};

use Kind::*;

/// How an expression which cannot be parsed is recovered from.
///
/// This is a plain enum rather than a trait object so that it can be stored in
/// the frames of the iterative parser.
#[derive(Debug, Clone, Copy)]
enum Cx {
    /// Perform no recovery, erroring out instead.
    Error,
    /// Recover to the end of the current statement.
    Stmt,
}

impl Cx {
    fn recover(self, p: &mut Parser<'_>) -> Result<()> {
        match self {
            Cx::Error => Err(p.expected_at(0, Kind::Expr)?),
            Cx::Stmt => {
                while is_stmt_recovering(p)? {
                    p.bump()?;
                }

                Ok(())
            }
        }
    }
}

/// Test if the parser is at a token which statement recovery should skip.
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
        // Glued, because that is what `attributes` and `inner_attributes`
        // require in order to consume the `#`. Looking past whitespace here
        // made this stop for a `#` which neither of them would then take, and
        // recovery which stops without consuming anything does not recover -
        // it spins. The two only differ when whitespace is part of the input,
        // which is how the formatter parses and the compiler does not.
        K![#] => !matches!(p.glued(1)?, K!['['] | K![!]),
        _ => true,
    };

    Ok(is)
}

#[derive(Default)]
struct Modifiers {
    is_pub: bool,
    is_const: bool,
    is_static: bool,
    is_async: bool,
}

#[derive(Debug, Clone, Copy)]
enum Brace {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy)]
enum Binary {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy)]
enum Range {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy)]
enum Callable {
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

    p.flush_ws()?;
    p.close()?;
    Ok(())
}

/// Parse comma-separated expressions.
pub(super) fn exprs(p: &mut Parser<'_>, separator: Kind) -> Result<()> {
    p.open(Root)?;

    while !p.is_eof()? {
        expr(p)?;
        p.bump_while(separator)?;
    }

    p.flush_ws()?;
    p.close()?;
    Ok(())
}

/// Parse the contents of an attribute, which is a path followed by the raw
/// token stream which is passed along to attribute macros as its input.
pub(super) fn attribute(p: &mut Parser<'_>) -> Result<()> {
    p.open(Root)?;

    path(p)?;

    let c = p.checkpoint()?;

    while !p.is_eof()? {
        p.bump()?;
    }

    p.close_at(&c, TokenStream)?;
    p.flush_ws()?;
    p.close()?;
    Ok(())
}

/// Parse comma-separated expressions.
pub(super) fn format(p: &mut Parser<'_>) -> Result<()> {
    p.open(Root)?;
    expr(p)?;

    while !p.is_eof()? {
        p.bump()?;
    }

    p.flush_ws()?;
    p.close()?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn inner_attributes(p: &mut Parser<'_>) -> Result<()> {
    while matches!((p.peek()?, p.glued(1)?), (K![#], K![!])) {
        let c = p.checkpoint()?;

        p.bump()?;
        p.bump_if(K![!])?;

        if p.bump_if(K!['['])? {
            token_stream(p, brackets, K![']'])?;
            p.bump()?;
        }

        p.close_at(&c, InnerAttribute)?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
fn attributes(p: &mut Parser<'_>) -> Result<()> {
    while matches!((p.peek()?, p.glued(1)?), (K![#], K!['['])) {
        let c = p.checkpoint()?;

        p.bump()?;

        if p.bump_if(K!['['])? {
            token_stream(p, brackets, K![']'])?;
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
                        K![super] => {
                            p.bump()?;
                            ModifierSuper
                        }
                        K![self] => {
                            p.bump()?;
                            ModifierSelf
                        }
                        K![crate] => {
                            p.bump()?;
                            ModifierCrate
                        }
                        K![in] => {
                            p.bump()?;
                            path(p)?;
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
            K![static] => {
                mods.is_static = true;
                p.bump()?;
            }
            K![async] => {
                mods.is_async = true;
                p.bump()?;
            }
            K![move] => {
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

enum UseStep {
    /// Close the path of an item in a group, and consume what separates it from
    /// the next one.
    Item { c: Checkpoint },
    /// Parse the next item of the group opened at the checkpoint, or close it.
    Group { c: Checkpoint },
}

/// Imports nest through groups, as in `use a::{b::{c, d}, e}`.
///
/// The nesting is walked over an explicit stack rather than recursively, so
/// that a deeply nested import costs heap rather than call stack. Nothing
/// downstream recurses over it - the worker processes a group over a queue -
/// so no limit is imposed on it here.
#[tracing::instrument(skip_all)]
fn item_use(p: &mut Parser<'_>) -> Result<()> {
    // Span used to attribute an allocation failure. The import is parsed from
    // this point onwards, so it is the best position available.
    let span = p.flush_ws()?;

    let mut stack = Vec::new();

    // The `use` keyword itself.
    p.bump()?;
    item_use_path(p, &mut stack, span)?;

    while let Some(step) = stack.pop() {
        match step {
            UseStep::Item { c } => {
                p.close_at(&c, ItemUsePath)?;
                p.bump_while(K![,])?;
            }
            UseStep::Group { c } => {
                if matches!(p.peek()?, path_component!() | K!['{'] | K![*]) {
                    let inner = p.checkpoint()?;

                    stack.try_push(UseStep::Group { c }).with_span(span)?;
                    stack.try_push(UseStep::Item { c: inner }).with_span(span)?;

                    // The leading token of the item in the group.
                    p.bump()?;
                    item_use_path(p, &mut stack, span)?;
                } else {
                    p.bump_if(K!['}'])?;
                    p.close_at(&c, ItemUseGroup)?;

                    // The path the group belongs to may continue after it.
                    item_use_path(p, &mut stack, span)?;
                }
            }
        }
    }

    Ok(())
}

/// Parse the components of a single import path, stopping when a group is
/// opened, which is pushed as a step for [`item_use`] to continue with.
fn item_use_path(p: &mut Parser<'_>, stack: &mut Vec<UseStep>, span: Span) -> Result<()> {
    loop {
        match p.peek()? {
            path_component!() | K![*] => {
                p.bump()?;
            }
            K!['{'] => {
                let c = p.checkpoint()?;
                p.bump()?;
                stack.try_push(UseStep::Group { c }).with_span(span)?;
                return Ok(());
            }
            K![as] => {
                p.bump()?;
                p.bump_if_matches(|k| matches!(k, K![ident]))?;
            }
            _ => return Ok(()),
        }
    }
}

#[tracing::instrument(skip_all)]
fn item_enum(p: &mut Parser<'_>) -> Result<()> {
    p.bump()?;

    if matches!(p.peek()?, K![ident]) {
        p.bump()?;
    }

    if p.bump_if(K!['{'])? {
        while matches!(p.peek()?, K![ident] | K![#]) {
            let variant = p.checkpoint()?;

            attributes(p)?;

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

            p.close_at(&variant, Variant)?;
            p.bump_while(K![,])?;
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

    while matches!(p.peek()?, K![ident] | K![#]) {
        let c = p.checkpoint()?;
        attributes(p)?;
        let field = p.nth_token(0)?;
        p.bump()?;

        // Rune has no field types, so an annotation is reported as the thing it
        // is rather than as a missing `}`.
        if p.peek()? == K![:] {
            p.bump()?;
            return Err(p.error(field.span, ErrorKind::msg(STATIC_TYPING_FIELD))?);
        }

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

    while matches!(p.peek()?, K![ident] | K![#]) {
        let c = p.checkpoint()?;
        attributes(p)?;
        p.bump()?;
        p.close_at(&c, Field)?;
        p.bump_while(K![,])?;
    }

    p.bump_if(K![')'])?;
    p.close_at(&c, TupleBody)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn is_pat(p: &mut Parser<'_>) -> Result<bool> {
    Ok(match p.peek()? {
        path_component!() => true,
        lit!() => true,
        K![_] => true,
        K!['('] => true,
        K!['['] => true,
        K![-] => matches!(p.glued(1)?, K![number]),
        K![#] => matches!(p.glued(1)?, K!['{']),
        _ => false,
    })
}

/// A pending step in the iterative pattern parser.
enum PatStep {
    /// Close the pattern opened at the checkpoint and leave its nesting level.
    Pat { c: Checkpoint },
    /// Close the node opened at the checkpoint once its children are parsed.
    Inner { c: Checkpoint, kind: Kind },
    /// Parse what remains of a sequence pattern, such as `(a, b)` or `[a, b]`.
    Seq { end: Kind, first: bool },
    /// Parse what remains of an object pattern.
    Object { after_value: bool },
}

/// Patterns nest through tuple, object and sequence patterns.
///
/// The nesting is walked over an explicit stack rather than recursively, so
/// that a deeply nested pattern costs heap rather than call stack. The nesting
/// limit is still accounted for, since the passes which consume the resulting
/// tree recurse over the same nesting.
#[tracing::instrument(skip_all)]
fn pat(p: &mut Parser<'_>) -> Result<()> {
    // Span used to attribute an allocation failure. Patterns are parsed from
    // this point onwards, so it is the best position available.
    let span = p.flush_ws()?;

    let mut stack = Vec::new();
    let mut start = true;

    loop {
        if start {
            start = false;
            pat_start(p, &mut stack, span)?;
            continue;
        }

        let Some(step) = stack.pop() else {
            break;
        };

        match step {
            PatStep::Pat { c } => {
                p.close_at(&c, Pat)?;
                p.leave();
            }
            PatStep::Inner { c, kind } => {
                p.close_at(&c, kind)?;
            }
            PatStep::Seq { end, first } => {
                if !first {
                    p.bump_while(K![,])?;
                }

                if is_pat(p)? || p.peek()? == K![..] {
                    stack
                        .try_push(PatStep::Seq { end, first: false })
                        .with_span(span)?;

                    if p.peek()? == K![..] {
                        p.bump()?;
                    } else {
                        start = true;
                    }
                } else {
                    p.bump_if(end)?;
                }
            }
            PatStep::Object { after_value } => {
                if after_value {
                    p.bump_while(K![,])?;
                }

                match p.peek()? {
                    K![..] => {
                        p.bump()?;

                        stack
                            .try_push(PatStep::Object { after_value: false })
                            .with_span(span)?;
                    }
                    object_key!() => {
                        p.bump()?;

                        stack
                            .try_push(PatStep::Object { after_value: true })
                            .with_span(span)?;

                        if p.bump_if(K![:])? {
                            start = true;
                        }
                    }
                    _ => {
                        p.bump_if(K!['}'])?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse the head of a single pattern, pushing the steps needed to finish it.
fn pat_start(p: &mut Parser<'_>, stack: &mut Vec<PatStep>, span: Span) -> Result<()> {
    p.enter()?;

    let c = p.checkpoint()?;
    attributes(p)?;

    stack.try_push(PatStep::Pat { c }).with_span(span)?;

    match p.peek()? {
        lit!() => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, Lit)?;
        }
        K![-] => {
            let c = p.checkpoint()?;
            p.bump()?;

            if matches!(p.peek()?, K![number]) {
                p.bump()?;
            }

            p.close_at(&c, Lit)?;
        }
        K![_] => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, PatIgnore)?;
        }
        path_component!() => {
            let c = p.checkpoint()?;
            path(p)?;

            match p.peek()? {
                K!['{'] => {
                    stack
                        .try_push(PatStep::Inner { c, kind: PatObject })
                        .with_span(span)?;
                    p.bump()?;
                    stack
                        .try_push(PatStep::Object { after_value: false })
                        .with_span(span)?;
                }
                K!['('] => {
                    stack
                        .try_push(PatStep::Inner { c, kind: PatTuple })
                        .with_span(span)?;
                    p.bump()?;
                    stack
                        .try_push(PatStep::Seq {
                            end: K![')'],
                            first: true,
                        })
                        .with_span(span)?;
                }
                _ => {}
            }
        }
        K!['['] => {
            let c = p.checkpoint()?;
            stack
                .try_push(PatStep::Inner { c, kind: PatArray })
                .with_span(span)?;
            p.bump()?;
            stack
                .try_push(PatStep::Seq {
                    end: K![']'],
                    first: true,
                })
                .with_span(span)?;
        }
        K!['('] => {
            let c = p.checkpoint()?;
            stack
                .try_push(PatStep::Inner { c, kind: PatTuple })
                .with_span(span)?;
            p.bump()?;
            stack
                .try_push(PatStep::Seq {
                    end: K![')'],
                    first: true,
                })
                .with_span(span)?;
        }
        K![#] if matches!(p.glued(1)?, K!['{']) => {
            let c = p.checkpoint()?;
            p.bump()?;
            p.close_at(&c, AnonymousObjectKey)?;
            stack
                .try_push(PatStep::Inner { c, kind: PatObject })
                .with_span(span)?;
            p.bump()?;
            stack
                .try_push(PatStep::Object { after_value: false })
                .with_span(span)?;
        }
        _ => {
            let c = p.checkpoint()?;
            p.close_at(&c, Error)?;
        }
    }

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
        Kind::Open(Delimiter::Empty) => true,
        K!['('] => true,
        K!['['] => true,
        K!['{'] => matches!(brace, Brace::Yes),
        K![..] => matches!(range, Range::Yes),
        K![..=] => matches!(range, Range::Yes),
        TemplateString => true,
        _ => false,
    })
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

fn labels(p: &mut Parser<'_>) -> Result<()> {
    while matches!(p.peek()?, K!['label]) {
        p.bump()?;
        p.bump_while(K![:])?;
    }

    Ok(())
}

/// Configuration for parsing an expression.
#[derive(Debug, Clone, Copy)]
struct Cfg {
    brace: Brace,
    range: Range,
    binary: Binary,
    callable: Callable,
    cx: Cx,
}

/// The configuration of a plain expression.
const EXPR: Cfg = Cfg {
    brace: Brace::Yes,
    range: Range::Yes,
    binary: Binary::Yes,
    callable: Callable::Yes,
    cx: Cx::Error,
};

/// The configuration of the head of a construct which is followed by a block,
/// where a brace opens that block rather than an object literal.
const HEAD: Cfg = Cfg {
    brace: Brace::No,
    range: Range::Yes,
    binary: Binary::Yes,
    callable: Callable::Yes,
    cx: Cx::Error,
};

/// Something which the parser has yet to parse.
#[derive(Debug, Clone, Copy)]
enum Task {
    /// An expression wrapped in an `Expr` node.
    Expr(Cfg),
    /// An expression which is not wrapped in an `Expr` node.
    Outer(Cfg),
    /// A block wrapped in a `Block` node.
    Block,
    /// The body of a block, `{ .. }`.
    BlockBody,
    /// A statement.
    Stmt,
}

/// A pending step in the iterative expression parser.
///
/// Expressions, blocks, statements and the items which contain blocks all nest
/// through each other, so rather than recursing they are driven from [`parse`]
/// with the work which remains kept on a heap allocated stack.
enum Step {
    /// Close the node opened at the checkpoint, making it the current kind.
    Close { c: Checkpoint, kind: Kind },
    /// Leave a nesting level.
    Leave,
    /// Parse another task once the current one completes.
    Then { task: Task },
    /// Continue an expression once its primary part has been parsed.
    Tail { c: Checkpoint, cfg: Cfg },
    /// Continue a binary expression once an operand has been parsed.
    Binary { state: BinaryState },
    /// Continue an expression with the range which may follow it, once any
    /// binary operators have been parsed.
    TailRange { c: Checkpoint, cfg: Cfg },
    /// Continue the postfix chain applied to an expression.
    Chain {
        c: Checkpoint,
        cfg: Cfg,
        before: Checkpoint,
        kind: Kind,
        chained: bool,
    },
    /// Finish the index element of a chain, `[expr]`.
    Index {
        c: Checkpoint,
        cfg: Cfg,
        before: Checkpoint,
    },
    /// Continue the arguments of the call element of a chain.
    Call {
        c: Checkpoint,
        cfg: Cfg,
        before: Checkpoint,
        first: bool,
    },
    /// Continue the elements of a group or a tuple, `( .. )`.
    Group { c: Checkpoint, tuple: bool },
    /// Continue the elements of an array, `[ .. ]`.
    Array { first: bool },
    /// Continue the fields of an object.
    Object { value: bool },
    /// Continue the `else` chain of an `if`.
    Else,
    /// Consume the given token if it is present.
    BumpIf { kind: Kind },
    /// Open the body of a `match` once its subject has been parsed.
    MatchOpen,
    /// Continue the arms of a `match`.
    MatchArms,
    /// Continue a match arm which has just parsed its guard.
    MatchGuard { c: Checkpoint },
    /// Close a match arm which has just parsed its body.
    MatchArm { c: Checkpoint },
    /// Continue the arms of a `select`.
    SelectArms,
    /// Continue a select arm which has just parsed its value.
    SelectValue { c: Checkpoint },
    /// Close a select arm which has just parsed its body.
    SelectArm { c: Checkpoint },
    /// Continue the statements of the block body opened at the checkpoint.
    Stmts { c: Checkpoint },
    /// Close a statement at the checkpoint.
    StmtEnd { c: Checkpoint, kind: Kind },
    /// Close a statement which is an expression at the checkpoint.
    StmtExpr { c: Checkpoint },
}

fn push(stack: &mut Vec<Step>, span: Span, step: Step) -> Result<()> {
    stack.try_push(step).with_span(span)?;
    Ok(())
}

/// Parse an expression wrapped in an `Expr` node.
fn expr(p: &mut Parser<'_>) -> Result<()> {
    parse(p, Task::Expr(EXPR))?;
    Ok(())
}

/// Parse a statement.
fn stmt(p: &mut Parser<'_>) -> Result<()> {
    parse(p, Task::Stmt)?;
    Ok(())
}

/// Drive a task to completion, returning the kind of the expression it ended
/// with.
///
/// Nesting is kept on `stack` rather than on the call stack. The nesting limit
/// is still accounted for, since the passes which consume the resulting tree
/// recurse over the same nesting.
fn parse(p: &mut Parser<'_>, task: Task) -> Result<Kind> {
    // Span used to attribute an allocation failure. Parsing proceeds from this
    // point onwards, so it is the best position available.
    let span = p.flush_ws()?;

    let mut stack = Vec::new();
    let mut next = Some(task);
    // The kind of the most recently completed expression.
    let mut kind = Error;

    loop {
        if let Some(task) = next.take() {
            match task {
                Task::Expr(cfg) => {
                    let c = p.checkpoint()?;
                    attributes(p)?;
                    modifiers(p)?;
                    labels(p)?;
                    push(&mut stack, span, Step::Close { c, kind: Expr })?;
                    next = Some(Task::Outer(cfg));
                }
                Task::Outer(cfg) => {
                    p.enter()?;
                    push(&mut stack, span, Step::Leave)?;
                    let c = p.checkpoint()?;
                    push(&mut stack, span, Step::Tail { c, cfg })?;
                    next = primary(p, &mut stack, span, cfg, &mut kind)?;
                }
                Task::Block => {
                    let c = p.checkpoint()?;
                    push(&mut stack, span, Step::Close { c, kind: Block })?;
                    next = Some(Task::BlockBody);
                }
                Task::BlockBody => {
                    p.enter()?;
                    push(&mut stack, span, Step::Leave)?;

                    if p.bump_if(K!['{'])? {
                        let c = p.checkpoint()?;
                        push(&mut stack, span, Step::Stmts { c })?;
                    } else {
                        p.bump_if(K!['}'])?;
                    }
                }
                Task::Stmt => {
                    next = stmt_start(p, &mut stack, span)?;
                }
            }

            continue;
        }

        let Some(step) = stack.pop() else {
            break;
        };

        match step {
            Step::Close { c, kind: k } => {
                p.close_at(&c, k)?;
                kind = k;
            }
            Step::Leave => {
                p.leave();
            }
            Step::Then { task } => {
                next = Some(task);
            }
            Step::Tail { c, cfg } => {
                if !is_range(kind) {
                    let before = p.checkpoint()?;

                    push(
                        &mut stack,
                        span,
                        Step::Chain {
                            c,
                            cfg,
                            before,
                            kind,
                            chained: false,
                        },
                    )?;
                }
            }
            Step::Binary { mut state } => {
                if let Some(task) = binary(p, &mut state)? {
                    push(&mut stack, span, Step::Binary { state })?;
                    next = Some(task);
                } else if state.has_any {
                    p.close_at(&state.expr, ExprBinary)?;
                    kind = ExprBinary;
                }
            }
            Step::TailRange { c, cfg } => {
                next = tail_range(p, &mut stack, span, c, cfg, &mut kind)?;
            }
            Step::Chain {
                c,
                cfg,
                before,
                kind: k,
                chained,
            } => {
                next = chain(p, &mut stack, span, c, cfg, before, k, chained, &mut kind)?;
            }
            Step::Index { c, cfg, before } => {
                p.bump_if(K![']'])?;
                chain_element(p, &mut stack, span, c, cfg, before, ExprIndex)?;
            }
            Step::Call {
                c,
                cfg,
                before,
                first,
            } => {
                if !first {
                    p.bump_while(K![,])?;
                }

                if is_expr(p)? {
                    push(
                        &mut stack,
                        span,
                        Step::Call {
                            c,
                            cfg,
                            before,
                            first: false,
                        },
                    )?;

                    next = Some(Task::Expr(EXPR));
                } else {
                    p.bump_if(K![')'])?;
                    chain_element(p, &mut stack, span, c, cfg, before, ExprCall)?;
                }
            }
            Step::Group { c, tuple } => {
                let tuple = p.bump_while(K![,])? || tuple;

                if is_expr(p)? {
                    push(&mut stack, span, Step::Group { c, tuple })?;
                    next = Some(Task::Expr(EXPR));
                } else {
                    p.bump_if(K![')'])?;
                    let k = if tuple { ExprTuple } else { ExprGroup };
                    p.close_at(&c, k)?;
                    kind = k;
                }
            }
            Step::Array { first } => {
                if !first {
                    p.bump_while(K![,])?;
                }

                if is_expr(p)? {
                    push(&mut stack, span, Step::Array { first: false })?;
                    next = Some(Task::Expr(EXPR));
                } else {
                    p.bump_if(K![']'])?;
                }
            }
            Step::Object { value } => {
                if value {
                    p.bump_while(K![,])?;
                }

                if matches!(p.peek()?, object_key!()) {
                    p.bump()?;
                    push(&mut stack, span, Step::Object { value: true })?;

                    if p.bump_if(K![:])? {
                        next = Some(Task::Expr(EXPR));
                    }
                } else {
                    p.bump_if(K!['}'])?;
                }
            }
            Step::Else => {
                if p.peek()? == K![else] {
                    let c = p.checkpoint()?;
                    p.bump()?;
                    push(&mut stack, span, Step::Else)?;

                    if p.bump_if(K![if])? {
                        push(
                            &mut stack,
                            span,
                            Step::Close {
                                c,
                                kind: ExprElseIf,
                            },
                        )?;
                        push(&mut stack, span, Step::Then { task: Task::Block })?;
                        next = condition(p, &mut stack, span)?;
                    } else {
                        push(&mut stack, span, Step::Close { c, kind: ExprElse })?;
                        next = Some(Task::Block);
                    }
                }
            }
            Step::BumpIf { kind: k } => {
                p.bump_if(k)?;
            }
            Step::MatchOpen => {
                if matches!(p.peek()?, K!['{']) {
                    p.bump()?;
                    push(&mut stack, span, Step::MatchArms)?;
                }
            }
            Step::MatchArms => {
                if is_pat(p)? {
                    let c = p.checkpoint()?;
                    pat(p)?;

                    if p.bump_if(K![if])? {
                        push(&mut stack, span, Step::MatchGuard { c })?;
                        next = Some(Task::Expr(EXPR));
                    } else if p.bump_if(K![=>])? {
                        push(&mut stack, span, Step::MatchArm { c })?;
                        next = Some(Task::Expr(EXPR));
                    } else {
                        p.close_at(&c, ExprMatchArm)?;
                        p.bump_while(K![,])?;
                        push(&mut stack, span, Step::MatchArms)?;
                    }
                } else {
                    p.bump_if(K!['}'])?;
                }
            }
            Step::MatchGuard { c } => {
                if p.bump_if(K![=>])? {
                    push(&mut stack, span, Step::MatchArm { c })?;
                    next = Some(Task::Expr(EXPR));
                } else {
                    p.close_at(&c, ExprMatchArm)?;
                    p.bump_while(K![,])?;
                    push(&mut stack, span, Step::MatchArms)?;
                }
            }
            Step::MatchArm { c } => {
                p.close_at(&c, ExprMatchArm)?;
                p.bump_while(K![,])?;
                push(&mut stack, span, Step::MatchArms)?;
            }
            Step::SelectArms => {
                if is_pat(p)? || p.peek()? == K![default] {
                    let c = p.checkpoint()?;

                    if p.peek()? == K![default] {
                        p.bump()?;
                    } else {
                        pat(p)?;

                        if p.bump_if(K![=])? {
                            push(&mut stack, span, Step::SelectValue { c })?;
                            next = Some(Task::Expr(EXPR));
                            continue;
                        }
                    }

                    if p.bump_if(K![=>])? {
                        push(&mut stack, span, Step::SelectArm { c })?;
                        next = Some(Task::Expr(EXPR));
                    } else {
                        p.close_at(&c, ExprSelectArm)?;
                        p.bump_while(K![,])?;
                        push(&mut stack, span, Step::SelectArms)?;
                    }
                } else {
                    p.bump_if(K!['}'])?;
                }
            }
            Step::SelectValue { c } => {
                if p.bump_if(K![=>])? {
                    push(&mut stack, span, Step::SelectArm { c })?;
                    next = Some(Task::Expr(EXPR));
                } else {
                    p.close_at(&c, ExprSelectArm)?;
                    p.bump_while(K![,])?;
                    push(&mut stack, span, Step::SelectArms)?;
                }
            }
            Step::SelectArm { c } => {
                p.close_at(&c, ExprSelectArm)?;
                p.bump_while(K![,])?;
                push(&mut stack, span, Step::SelectArms)?;
            }
            Step::Stmts { c } => {
                if !matches!(p.peek()?, K!['}'] | Eof) {
                    push(&mut stack, span, Step::Stmts { c })?;
                    next = Some(Task::Stmt);
                } else {
                    p.close_at(&c, BlockBody)?;
                    p.bump_if(K!['}'])?;
                }
            }
            Step::StmtEnd { c, kind: k } => {
                p.close_at(&c, k)?;
                p.bump_while(K![;])?;
            }
            Step::StmtExpr { c } => {
                let k = if matches!(kind, Error) { Error } else { Expr };
                p.close_at(&c, k)?;
                p.bump_while(K![;])?;
            }
        }
    }

    Ok(kind)
}

/// Close a completed chain element and queue the rest of the chain.
fn chain_element(
    p: &mut Parser<'_>,
    stack: &mut Vec<Step>,
    span: Span,
    c: Checkpoint,
    cfg: Cfg,
    before: Checkpoint,
    kind: Kind,
) -> Result<()> {
    p.close_at(&before, kind)?;
    let before = p.checkpoint()?;

    push(
        stack,
        span,
        Step::Chain {
            c,
            cfg,
            before,
            kind,
            chained: true,
        },
    )
}

/// Perform a single step of a postfix chain.
#[allow(clippy::too_many_arguments)]
fn chain(
    p: &mut Parser<'_>,
    stack: &mut Vec<Step>,
    span: Span,
    c: Checkpoint,
    cfg: Cfg,
    before: Checkpoint,
    kind: Kind,
    chained: bool,
    out: &mut Kind,
) -> Result<Option<Task>> {
    if !p.is_eof()? {
        let is_callable = kind_is_callable(kind, cfg.callable);

        match p.peek()? {
            K!['['] if is_callable => {
                p.bump()?;

                push(stack, span, Step::Index { c, cfg, before })?;

                return Ok(Some(Task::Expr(EXPR)));
            }
            K!['('] if is_callable => {
                p.bump()?;

                push(
                    stack,
                    span,
                    Step::Call {
                        c,
                        cfg,
                        before,
                        first: true,
                    },
                )?;

                return Ok(None);
            }
            K![?] => {
                p.bump()?;
                chain_element(p, stack, span, c, cfg, before, ExprTry)?;
                return Ok(None);
            }
            K![.] => {
                p.bump()?;

                let k = match p.peek()? {
                    K![await] => {
                        p.bump()?;
                        ExprAwait
                    }
                    path_component!() => {
                        path(p)?;
                        ExprField
                    }
                    K![number] => {
                        p.bump()?;
                        ExprField
                    }
                    _ => Error,
                };

                chain_element(p, stack, span, c, cfg, before, k)?;
                return Ok(None);
            }
            _ => {}
        }
    }

    if chained {
        p.close_at(&c, ExprChain)?;
    }

    *out = kind;

    // A block-like expression written where a statement is expected is a
    // statement in its own right, so what follows it begins the next one.
    //
    // Without this it takes what follows as its own tail, so the `-1` in
    // `if a { return 1; } -1` was a subtraction from the value of the `if`
    // rather than the value the block ends with.
    if !chained && matches!(cfg.callable, Callable::No) && kind_is_block_like(kind) {
        return Ok(None);
    }

    tail(p, stack, span, c, cfg, out)
}

/// Parse what follows the chain of an expression - assignment, binary
/// operators and ranges.
fn tail(
    p: &mut Parser<'_>,
    stack: &mut Vec<Step>,
    span: Span,
    c: Checkpoint,
    cfg: Cfg,
    out: &mut Kind,
) -> Result<Option<Task>> {
    if p.peek()? == K![=] {
        p.close_at(&c, Expr)?;
        p.bump()?;

        push(
            stack,
            span,
            Step::Close {
                c,
                kind: ExprAssign,
            },
        )?;

        return Ok(Some(Task::Expr(Cfg {
            brace: cfg.brace,
            range: Range::Yes,
            binary: Binary::Yes,
            callable: Callable::Yes,
            cx: cfg.cx,
        })));
    }

    if matches!(cfg.binary, Binary::Yes) {
        let mut state = BinaryState {
            start: c.span(),
            expr: c.clone(),
            cfg,
            levels: Vec::new(),
            has_any: false,
        };

        if let Some(task) = binary(p, &mut state)? {
            // The range which may follow is parsed once the binary expression
            // is complete, so it is pushed under it.
            push(stack, span, Step::TailRange { c, cfg })?;
            push(stack, span, Step::Binary { state })?;
            return Ok(Some(task));
        }
    }

    tail_range(p, stack, span, c, cfg, out)
}

/// Parse the range which may follow an expression.
fn tail_range(
    p: &mut Parser<'_>,
    stack: &mut Vec<Step>,
    span: Span,
    c: Checkpoint,
    cfg: Cfg,
    out: &mut Kind,
) -> Result<Option<Task>> {
    if matches!(cfg.range, Range::Yes) {
        let operand = Cfg {
            brace: cfg.brace,
            range: Range::No,
            binary: Binary::Yes,
            callable: Callable::Yes,
            cx: Cx::Error,
        };

        match p.peek()? {
            K![..] => {
                p.bump()?;

                if is_expr_with(p, cfg.brace, Range::No)? {
                    push(stack, span, Step::Close { c, kind: ExprRange })?;

                    return Ok(Some(Task::Outer(operand)));
                }

                p.close_at(&c, ExprRangeFrom)?;
                *out = ExprRangeFrom;
            }
            K![..=] => {
                p.bump()?;

                if is_expr_with(p, cfg.brace, Range::No)? {
                    push(
                        stack,
                        span,
                        Step::Close {
                            c,
                            kind: ExprRangeInclusive,
                        },
                    )?;

                    return Ok(Some(Task::Outer(operand)));
                }

                return Err(p.error(c.span(), ErrorKind::msg(UNSUPPORTED_RANGE))?);
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Reported for an inclusive range which is missing the end it has to include.
const UNSUPPORTED_RANGE: &str = "Unsupported range, you probably want `..` instead of `..=`";

/// Reported for a type annotation on a struct field.
const STATIC_TYPING_FIELD: &str = "Static typing on fields is not supported";

/// Reported for a return type annotation on a function.
const STATIC_TYPING_RETURN: &str = "Adding a return type in functions is not supported";

/// Reject the `mut` modifier, which a binding may not carry.
///
/// The grammar has no production for it, so without this it would be reported
/// as whatever the parser expected in its place rather than as the thing the
/// author actually wrote.
fn deny_mut(p: &mut Parser<'_>) -> Result<()> {
    if p.peek()? == K![mut] {
        let tok = p.nth_token(0)?;
        return Err(Error::new(tok.span, ErrorKind::UnsupportedMut));
    }

    Ok(())
}

/// Parse the condition of an `if` or `while`.
fn condition(p: &mut Parser<'_>, stack: &mut Vec<Step>, span: Span) -> Result<Option<Task>> {
    if p.peek()? == K![let] {
        let c = p.checkpoint()?;
        p.bump()?;
        deny_mut(p)?;
        pat(p)?;

        if p.peek()? == K![=] {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: Condition })?;
            return Ok(Some(Task::Expr(HEAD)));
        }

        p.close_at(&c, Condition)?;
        return Ok(None);
    }

    Ok(Some(Task::Expr(HEAD)))
}

/// Parse the primary part of an expression, pushing the steps needed to finish
/// it.
fn primary(
    p: &mut Parser<'_>,
    stack: &mut Vec<Step>,
    span: Span,
    cfg: Cfg,
    kind: &mut Kind,
) -> Result<Option<Task>> {
    let c = p.checkpoint()?;

    // The configuration of an operand which may not itself be a range.
    let operand = Cfg {
        brace: cfg.brace,
        range: Range::No,
        binary: Binary::Yes,
        callable: Callable::Yes,
        cx: Cx::Error,
    };

    // The configuration of a nested expression which inherits its position.
    let inner = Cfg {
        brace: cfg.brace,
        range: cfg.range,
        binary: Binary::Yes,
        callable: Callable::Yes,
        cx: cfg.cx,
    };

    match p.peek()? {
        lit!() => {
            p.bump()?;
            p.close_at(&c, Lit)?;
            *kind = Lit;
        }
        path_component!() => {
            path(p)?;

            match p.peek()? {
                K!['{'] if matches!(cfg.brace, Brace::Yes) => {
                    push(
                        stack,
                        span,
                        Step::Close {
                            c,
                            kind: ExprObject,
                        },
                    )?;
                    p.bump()?;
                    push(stack, span, Step::Object { value: false })?;
                }
                // A macro is invoked through any of the three delimiters, which
                // are interchangeable since its input is a raw token stream
                // either way.
                K![!] if matches!(p.glued(1)?, K!['('] | K!['['] | K!['{']) => {
                    let (matcher, close) = match p.glued(1)? {
                        K!['('] => (parens as fn(Kind) -> i32, K![')']),
                        K!['['] => (brackets as fn(Kind) -> i32, K![']']),
                        _ => (braces as fn(Kind) -> i32, K!['}']),
                    };

                    p.bump()?;
                    p.bump()?;
                    token_stream(p, matcher, close)?;
                    p.bump()?;
                    p.close_at(&c, ExprMacroCall)?;
                    *kind = ExprMacroCall;
                }
                _ => {
                    // NB: a bare path is not closed, matching what the
                    // expression is expected to look like downstream.
                    *kind = Path;
                }
            }
        }
        Kind::Open(Delimiter::Empty) => {
            push(
                stack,
                span,
                Step::Close {
                    c,
                    kind: ExprEmptyGroup,
                },
            )?;
            p.bump()?;
            push(
                stack,
                span,
                Step::BumpIf {
                    kind: Kind::Close(Delimiter::Empty),
                },
            )?;
            return Ok(Some(Task::Expr(EXPR)));
        }
        K!['('] => {
            p.bump()?;

            if is_expr(p)? {
                push(stack, span, Step::Group { c, tuple: false })?;
                return Ok(Some(Task::Expr(EXPR)));
            }

            p.bump_if(K![')'])?;
            p.close_at(&c, ExprGroup)?;
            *kind = ExprGroup;
        }
        K!['['] => {
            push(stack, span, Step::Close { c, kind: ExprArray })?;
            p.bump()?;
            push(stack, span, Step::Array { first: true })?;
        }
        K!['{'] => {
            push(stack, span, Step::Close { c, kind: Block })?;
            return Ok(Some(Task::BlockBody));
        }
        TemplateString => {
            p.bump()?;
            p.close_at(&c, TemplateString)?;
            *kind = TemplateString;
        }
        K![||] => {
            p.push(ClosureArguments)?;
            push(
                stack,
                span,
                Step::Close {
                    c,
                    kind: ExprClosure,
                },
            )?;
            return Ok(Some(Task::Expr(inner)));
        }
        K![|] => {
            let args = p.checkpoint()?;
            parenthesized(p, is_pat, pat, K![|])?;
            p.close_at(&args, ClosureArguments)?;
            push(
                stack,
                span,
                Step::Close {
                    c,
                    kind: ExprClosure,
                },
            )?;
            return Ok(Some(Task::Expr(inner)));
        }
        K![-] if matches!(p.glued(1)?, K![number]) => {
            p.bump()?;
            p.bump()?;
            p.close_at(&c, Lit)?;
            *kind = Lit;
        }
        K![!] | K![-] | K![&] | K![*] => {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: ExprUnary })?;

            return Ok(Some(Task::Outer(Cfg {
                brace: cfg.brace,
                range: cfg.range,
                binary: Binary::No,
                callable: Callable::Yes,
                cx: cfg.cx,
            })));
        }
        K![if] => {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: ExprIf })?;
            push(stack, span, Step::Else)?;
            push(stack, span, Step::Then { task: Task::Block })?;
            return condition(p, stack, span);
        }
        K![while] => {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: ExprWhile })?;
            push(stack, span, Step::Then { task: Task::Block })?;
            return condition(p, stack, span);
        }
        K![loop] => {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: ExprLoop })?;
            return Ok(Some(Task::Block));
        }
        K![break] => {
            p.bump()?;
            labels(p)?;

            if is_expr_with(p, cfg.brace, cfg.range)? {
                push(stack, span, Step::Close { c, kind: ExprBreak })?;

                return Ok(Some(Task::Expr(Cfg {
                    cx: Cx::Error,
                    ..inner
                })));
            }

            p.close_at(&c, ExprBreak)?;
            *kind = ExprBreak;
        }
        K![continue] => {
            p.bump()?;
            labels(p)?;
            p.close_at(&c, ExprContinue)?;
            *kind = ExprContinue;
        }
        K![return] => {
            p.bump()?;

            if is_expr_with(p, cfg.brace, cfg.range)? {
                push(
                    stack,
                    span,
                    Step::Close {
                        c,
                        kind: ExprReturn,
                    },
                )?;

                return Ok(Some(Task::Expr(Cfg {
                    cx: Cx::Error,
                    ..inner
                })));
            }

            p.close_at(&c, ExprReturn)?;
            *kind = ExprReturn;
        }
        K![yield] => {
            p.bump()?;

            if is_expr_with(p, cfg.brace, cfg.range)? {
                push(stack, span, Step::Close { c, kind: ExprYield })?;

                return Ok(Some(Task::Expr(Cfg {
                    cx: Cx::Error,
                    ..inner
                })));
            }

            p.close_at(&c, ExprYield)?;
            *kind = ExprYield;
        }
        K![for] => {
            p.bump()?;
            pat(p)?;
            push(stack, span, Step::Close { c, kind: ExprFor })?;
            push(stack, span, Step::Then { task: Task::Block })?;

            if p.bump_if(K![in])? {
                return Ok(Some(Task::Expr(HEAD)));
            }
        }
        K![select] => {
            p.bump()?;
            push(
                stack,
                span,
                Step::Close {
                    c,
                    kind: ExprSelect,
                },
            )?;

            if matches!(p.peek()?, K!['{']) {
                p.bump()?;
                push(stack, span, Step::SelectArms)?;
            }
        }
        K![match] => {
            p.bump()?;
            push(stack, span, Step::Close { c, kind: ExprMatch })?;
            push(stack, span, Step::MatchOpen)?;
            return Ok(Some(Task::Expr(HEAD)));
        }
        K![..] if matches!(cfg.range, Range::Yes) => {
            p.bump()?;

            if is_expr_with(p, cfg.brace, Range::No)? {
                push(
                    stack,
                    span,
                    Step::Close {
                        c,
                        kind: ExprRangeTo,
                    },
                )?;
                return Ok(Some(Task::Outer(operand)));
            }

            p.close_at(&c, ExprRangeFull)?;
            *kind = ExprRangeFull;
        }
        K![..=] if matches!(cfg.range, Range::Yes) => {
            p.bump()?;

            if is_expr_with(p, cfg.brace, Range::No)? {
                push(
                    stack,
                    span,
                    Step::Close {
                        c,
                        kind: ExprRangeToInclusive,
                    },
                )?;

                return Ok(Some(Task::Outer(operand)));
            }

            return Err(p.error(c.span(), ErrorKind::msg(UNSUPPORTED_RANGE))?);
        }
        K![#] if matches!(p.glued(1)?, K!['{']) => {
            let key = p.checkpoint()?;
            p.bump()?;
            p.close_at(&key, AnonymousObjectKey)?;
            push(
                stack,
                span,
                Step::Close {
                    c,
                    kind: ExprObject,
                },
            )?;
            p.bump()?;
            push(stack, span, Step::Object { value: false })?;
        }
        _ => {
            cfg.cx.recover(p)?;
            p.close_at(&c, Error)?;
            *kind = Error;
        }
    }

    Ok(None)
}

/// Parse the head of a statement, pushing the steps needed to finish it.
fn stmt_start(p: &mut Parser<'_>, stack: &mut Vec<Step>, span: Span) -> Result<Option<Task>> {
    inner_attributes(p)?;

    let c = p.checkpoint()?;
    attributes(p)?;

    let inner_c = p.checkpoint()?;
    let m = modifiers(p)?;

    match p.peek()? {
        K![let] => {
            push(stack, span, Step::StmtEnd { c, kind: Local })?;
            p.bump()?;
            deny_mut(p)?;
            pat(p)?;
            p.bump_if(K![=])?;

            Ok(Some(Task::Expr(Cfg {
                brace: Brace::Yes,
                range: Range::Yes,
                binary: Binary::Yes,
                callable: Callable::Yes,
                cx: Cx::Stmt,
            })))
        }
        K![use] => {
            item_use(p)?;
            p.close_at(&inner_c, ItemUse)?;
            p.close_at(&c, Item)?;
            p.bump_while(K![;])?;
            Ok(None)
        }
        K![struct] => {
            item_struct(p)?;
            p.close_at(&inner_c, ItemStruct)?;
            p.close_at(&c, Item)?;
            p.bump_while(K![;])?;
            Ok(None)
        }
        K![enum] => {
            item_enum(p)?;
            p.close_at(&inner_c, ItemEnum)?;
            p.close_at(&c, Item)?;
            p.bump_while(K![;])?;
            Ok(None)
        }
        K![fn] => {
            p.bump()?;

            if matches!(p.peek()?, K![ident]) {
                p.bump()?;
            }

            if p.peek()? == K!['('] {
                let args = p.checkpoint()?;
                p.bump()?;

                p.bump_while(K![,])?;

                while is_pat(p)? {
                    pat(p)?;
                    p.bump_while(K![,])?;
                }

                p.bump_if(K![')'])?;
                p.close_at(&args, FnArgs)?;
            }

            // Rune has no return types, so an annotation is reported as the
            // thing it is rather than as a missing body.
            if p.peek()? == K![->] {
                let arrow = p.nth_token(0)?;
                p.bump()?;
                return Err(p.error(arrow.span, ErrorKind::msg(STATIC_TYPING_RETURN))?);
            }

            push(stack, span, Step::StmtEnd { c, kind: Item })?;
            push(
                stack,
                span,
                Step::Close {
                    c: inner_c,
                    kind: ItemFn,
                },
            )?;

            if p.peek()? == K!['{'] {
                return Ok(Some(Task::Block));
            }

            Ok(None)
        }
        K![impl] => {
            p.bump()?;

            if matches!(p.peek()?, path_component!()) {
                path(p)?;
            }

            push(stack, span, Step::StmtEnd { c, kind: Item })?;
            push(
                stack,
                span,
                Step::Close {
                    c: inner_c,
                    kind: ItemImpl,
                },
            )?;

            Ok(Some(Task::Block))
        }
        K![mod] => {
            p.bump()?;

            if matches!(p.peek()?, K![ident]) {
                p.bump()?;
            }

            push(stack, span, Step::StmtEnd { c, kind: Item })?;

            if matches!(p.peek()?, K!['{']) {
                push(
                    stack,
                    span,
                    Step::Close {
                        c: inner_c,
                        kind: ItemMod,
                    },
                )?;

                return Ok(Some(Task::Block));
            }

            p.close_at(&inner_c, ItemFileMod)?;
            Ok(None)
        }
        K![ident] if m.is_const => {
            p.bump()?;
            p.bump_if(K![=])?;

            push(stack, span, Step::StmtEnd { c, kind: Item })?;
            push(
                stack,
                span,
                Step::Close {
                    c: inner_c,
                    kind: ItemConst,
                },
            )?;

            Ok(Some(Task::Expr(EXPR)))
        }
        K![ident] if m.is_static => {
            p.bump()?;

            push(stack, span, Step::StmtEnd { c, kind: Item })?;
            push(
                stack,
                span,
                Step::Close {
                    c: inner_c,
                    kind: ItemStatic,
                },
            )?;

            if p.bump_if(K![=])? {
                return Ok(Some(Task::Expr(EXPR)));
            }

            Ok(None)
        }
        _ => {
            labels(p)?;
            push(stack, span, Step::StmtExpr { c })?;

            Ok(Some(Task::Outer(Cfg {
                brace: Brace::Yes,
                range: Range::Yes,
                binary: Binary::Yes,
                callable: Callable::No,
                cx: Cx::Stmt,
            })))
        }
    }
}

/// Test if an expression is one written as a block.
///
/// These are the expressions which are a statement of their own when they are
/// written where a statement is expected, rather than the beginning of one.
fn kind_is_block_like(kind: Kind) -> bool {
    matches!(
        kind,
        ExprWhile | ExprLoop | ExprFor | ExprIf | ExprMatch | ExprSelect | Block
    )
}

fn kind_is_callable(kind: Kind, callable: Callable) -> bool {
    match kind {
        ExprWhile => false,
        ExprLoop => matches!(callable, Callable::Yes),
        ExprFor => false,
        ExprIf => matches!(callable, Callable::Yes),
        ExprMatch => matches!(callable, Callable::Yes),
        ExprSelect => matches!(callable, Callable::Yes),
        Block => matches!(callable, Callable::Yes),
        _ => true,
    }
}

/// A pending step in the iterative path parser.
enum PathStep {
    /// Close the path opened at the checkpoint.
    Path { c: Checkpoint },
    /// Continue the components of a path.
    Components,
    /// Continue the arguments of the generics opened at the checkpoint.
    Generics { c: Checkpoint, after: bool },
}

/// Paths nest through generic arguments - `a::<b::<c>>` - so they are walked
/// over an explicit stack rather than recursively.
#[tracing::instrument(skip_all)]
fn path(p: &mut Parser<'_>) -> Result<()> {
    // Span used to attribute an allocation failure.
    let span = p.flush_ws()?;

    let mut stack = Vec::new();
    let mut start = true;

    loop {
        if start {
            start = false;
            let c = p.checkpoint()?;
            stack.try_push(PathStep::Path { c }).with_span(span)?;
            stack.try_push(PathStep::Components).with_span(span)?;
            continue;
        }

        let Some(step) = stack.pop() else {
            break;
        };

        match step {
            PathStep::Path { c } => {
                p.close_at(&c, Path)?;
            }
            PathStep::Components => {
                if matches!(p.peek()?, path_component!()) {
                    // Parse a generic path if we are in a context supporting
                    // binary expressions, or if we just parsed the prefix `::`
                    // of the turbofish syntax.
                    let has_generics = matches!(p.peek()?, K![::]);
                    p.bump()?;

                    stack.try_push(PathStep::Components).with_span(span)?;

                    // We can't parse generics in binary expressions, since they
                    // would be ambiguous with expressions such as
                    // `self::FOO< 10`.
                    if has_generics && matches!(p.peek()?, K![<]) {
                        let c = p.checkpoint()?;
                        p.bump()?;

                        stack
                            .try_push(PathStep::Generics { c, after: false })
                            .with_span(span)?;
                    }
                }
            }
            PathStep::Generics { c, after } => {
                if after {
                    p.bump_while(K![,])?;
                }

                if matches!(p.peek()?, path_component!()) {
                    stack
                        .try_push(PathStep::Generics { c, after: true })
                        .with_span(span)?;

                    // Inner paths are unambiguous.
                    start = true;
                    continue;
                }

                p.bump_gt()?;
                p.close_at(&c, PathGenerics)?;

                if matches!(p.peek()?, K![<]) {
                    let c = p.checkpoint()?;
                    p.bump()?;

                    stack
                        .try_push(PathStep::Generics { c, after: false })
                        .with_span(span)?;
                }
            }
        }
    }

    Ok(())
}

/// One level of precedence climbing which is still open.
///
/// A level is opened by an operator and closed once the operators which follow
/// bind less tightly than it does.
#[derive(Clone)]
struct BinaryLevel {
    /// The lowest precedence this level accepts, which is one more than the
    /// precedence of the level it was opened from.
    min_precedence: usize,
    /// The precedence of the operator whose operand was parsed last.
    precedence: usize,
    /// The checkpoint just after that operator, which a sub-expression binding
    /// more tightly is closed at.
    c: Checkpoint,
}

/// The state of a binary expression which is being parsed.
///
/// Operands are parsed by the driver rather than by recursing, so that a chain
/// like `a + (b + (c + ..))` costs heap rather than call stack. Precedence
/// climbing is over `levels`, which is bounded by the number of precedences
/// anyway, but its operands are not.
struct BinaryState {
    /// The checkpoint of the expression the chain belongs to.
    expr: Checkpoint,
    /// Where the left-most operand starts, so that an operator which needs
    /// grouping is reported over the chain it appears in rather than over
    /// whichever level happened to notice it.
    start: Span,
    cfg: Cfg,
    /// The levels which are open, outermost first.
    levels: Vec<BinaryLevel>,
    /// Whether the outermost level parsed any operator at all.
    has_any: bool,
}

impl BinaryState {
    /// Consume an operator and open a level for the operand which follows it,
    /// which the driver parses before the state is advanced again.
    fn open(&mut self, p: &mut Parser<'_>, op: ast::BinOp, min_precedence: usize) -> Result<Task> {
        let op_c = p.checkpoint()?;
        op.advance(p)?;
        p.close_at(&op_c, ExprOperator)?;

        let c = p.checkpoint()?;

        self.levels
            .try_push(BinaryLevel {
                min_precedence,
                precedence: op.precedence(),
                c,
            })
            .with_span(self.start)?;

        let range = if op.is_assoc() { Range::No } else { Range::Yes };

        Ok(Task::Outer(Cfg {
            brace: self.cfg.brace,
            range,
            binary: Binary::No,
            callable: Callable::Yes,
            cx: self.cfg.cx,
        }))
    }
}

/// Peek the binary operator the parser is at, if any.
fn binary_op(p: &mut Parser<'_>) -> Result<Option<ast::BinOp>> {
    let slice = p.array::<2>()?;
    Ok(ast::BinOp::from_slice(&slice))
}

/// Advance a binary expression, returning the operand which has to be parsed
/// before it can be advanced again, or `None` once it is complete.
#[tracing::instrument(skip_all)]
fn binary(p: &mut Parser<'_>, state: &mut BinaryState) -> Result<Option<Task>> {
    loop {
        let Some(level) = state.levels.last().cloned() else {
            // The left-hand side has been parsed, so the first operator opens
            // the outermost level.
            let Some(op) = binary_op(p)? else {
                return Ok(None);
            };

            state.has_any = true;
            return Ok(Some(state.open(p, op, 0)?));
        };

        if let Some(next) = binary_op(p)? {
            let precedence = next.precedence();

            if level.precedence < precedence {
                // An operator which binds more tightly is parsed as a
                // sub-expression of the operand this level just parsed.
                return Ok(Some(state.open(p, next, level.precedence + 1)?));
            }

            if level.precedence == precedence && !next.is_assoc() {
                return Err(p.error(state.start, ErrorKind::PrecedenceGroupRequired)?);
            }

            if precedence >= level.min_precedence {
                // Another operand at the same level.
                state.levels.pop();
                return Ok(Some(state.open(p, next, level.min_precedence)?));
            }
        }

        // Nothing more binds at this level, so it is closed. The level it was
        // opened from closes it at the operand it applies to.
        state.levels.pop();

        let Some(parent) = state.levels.last() else {
            return Ok(None);
        };

        p.close_at(&parent.c, ExprBinary)?;
    }
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
///
/// The group has to be closed by the end of the input. There is nothing left to
/// consume once that is reached, so a group which is never closed has to be
/// reported here rather than consumed further.
#[tracing::instrument(skip_all)]
fn token_stream(p: &mut Parser<'_>, matcher: fn(Kind) -> i32, close: Kind) -> Result<()> {
    let c = p.checkpoint()?;

    let mut level = 1u32;

    while level > 0 {
        if p.is_eof()? {
            return Err(p.expected_at(0, close)?);
        }

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
