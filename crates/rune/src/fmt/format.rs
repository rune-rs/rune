use crate::alloc::Vec;
use crate::ast::{Delimiter, Kind};
use crate::compile::Result;
use crate::grammar::{classify, object_key, MaybeNode, NodeClass, StreamBuf};

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

/// A pending step in the iterative path formatter.
enum PathStep<'a> {
    /// Emit what remains of the components of a path.
    Path(StreamBuf<'a>),
    /// Emit what remains of the arguments of the generics of a path, with the
    /// separator which the argument last emitted was followed by.
    Generics(StreamBuf<'a>, Remaining<'a>),
    /// Close the argument which was just emitted, so that the separator after
    /// it is picked up before the next one is started.
    GenericsArg(StreamBuf<'a>),
}

/// Paths nest through generic arguments - `a::<b::<c>>` - which no limit
/// applies to, so they are walked over an explicit stack rather than by
/// recursing over how deeply they nest.
fn path<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut stack = Vec::new();
    stack.try_push(PathStep::Path(p.take_remaining()))?;

    while let Some(step) = stack.pop() {
        match step {
            PathStep::Path(mut buf) => {
                let Some(node) = buf.next() else {
                    buf.end()?;
                    continue;
                };

                if matches!(node.kind(), PathGenerics) {
                    stack.try_push(PathStep::Path(buf))?;

                    let mut buf = node.into_stream();
                    buf.stream().expect(K![<])?.fmt(fmt)?;
                    stack.try_push(PathStep::Generics(buf, Remaining::default()))?;
                } else {
                    node.fmt(fmt)?;
                    stack.try_push(PathStep::Path(buf))?;
                }
            }
            PathStep::Generics(mut buf, comma) => {
                if let MaybeNode::Some(node) = buf.stream().eat(Path) {
                    fmt.comments(Prefix)?;

                    if comma.fmt(fmt)? {
                        fmt.ws()?;
                    }

                    stack.try_push(PathStep::GenericsArg(buf))?;
                    stack.try_push(PathStep::Path(node.into_stream()))?;
                } else {
                    if !comma.ignore(fmt)? {
                        fmt.comments(Infix)?;
                    }

                    buf.stream().one(K![>]).fmt(fmt)?;
                    buf.end()?;
                }
            }
            PathStep::GenericsArg(mut buf) => {
                let comma = buf.stream().remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
                stack.try_push(PathStep::Generics(buf, comma))?;
            }
        }
    }

    Ok(())
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
        let mut last = None;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                last = Some(node.clone());
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
            last = Some(node.clone());
            fmt.write_raw(node)?;
            has_ws = false;
        }

        // What follows the stream is not part of it, so the comments there are
        // picked up the way they are anywhere else.
        if let Some(last) = last {
            fmt.ignore(last)?;
        }

        fmt.nl(1)?;
        fmt.indent(-1)?;
        Ok(())
    })?;

    p.expect(K!['}'])?.fmt(fmt)?;
    Ok(())
}

fn compact_expr_macro_call<'a>(
    fmt: &mut Formatter<'a>,
    p: &mut Stream<'a>,
    open: Kind,
    close: Kind,
) -> Result<()> {
    p.expect(open)?.fmt(fmt)?;

    p.expect(TokenStream)?.parse(|p| {
        let mut buf = None;
        let mut has_ws = false;
        let mut last = None;

        while let Some(node) = p.next_with_ws() {
            if matches!(node.kind(), K![,]) {
                last = Some(node.clone());
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
            last = Some(node.clone());
            fmt.write_raw(node)?;
            has_ws = false;
        }

        // What follows the stream is not part of it, so the comments there are
        // picked up the way they are anywhere else.
        if let Some(last) = last {
            fmt.ignore(last)?;
        }

        Ok(())
    })?;

    p.expect(close)?.fmt(fmt)?;
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

fn item_use<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    p.expect(K![use])?.fmt(fmt)?;
    fmt.ws()?;
    item_use_path(fmt, p)
}

/// A pending step in the iterative import formatter.
enum UseStep<'a> {
    /// Emit what remains of the components of an import path.
    Path(StreamBuf<'a>),
    /// Emit what remains of the items of a group, with the separator which the
    /// item last emitted was followed by.
    ///
    /// A group which holds a single item is written without its braces, which
    /// is what `braced` records.
    Group(StreamBuf<'a>, Remaining<'a>, bool),
    /// Close the item which was just emitted, so that the separator after it is
    /// picked up before the next one is started.
    GroupItem(StreamBuf<'a>, bool),
}

/// Imports nest through groups - `use a::{b::{c}}` - which no limit applies to,
/// so they are walked over an explicit stack rather than by recursing over how
/// deeply they nest.
fn item_use_path<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<()> {
    let mut stack = Vec::new();
    stack.try_push(UseStep::Path(p.take_remaining()))?;

    while let Some(step) = stack.pop() {
        match step {
            UseStep::Path(mut buf) => {
                let Some(node) = buf.next() else {
                    buf.end()?;
                    continue;
                };

                match node.kind() {
                    ItemUseGroup => {
                        stack.try_push(UseStep::Path(buf))?;

                        let mut buf = node.into_stream();
                        let braced = use_group_open(fmt, buf.stream())?;
                        stack.try_push(UseStep::Group(buf, Remaining::default(), braced))?;
                    }
                    K![as] => {
                        fmt.ws()?;
                        node.fmt(fmt)?;

                        if let MaybeNode::Some(node) =
                            buf.stream().eat_matching(|k| matches!(k, K![ident]))
                        {
                            fmt.ws()?;
                            node.fmt(fmt)?;
                        }

                        // Nothing may follow the alias, so what is left is
                        // reported the same way the enclosing node would have.
                        buf.end()?;
                    }
                    _ => {
                        node.fmt(fmt)?;
                        stack.try_push(UseStep::Path(buf))?;
                    }
                }
            }
            UseStep::Group(mut buf, comma, braced) => {
                if let MaybeNode::Some(inner) = buf.stream().eat(ItemUsePath) {
                    fmt.comments(Prefix)?;

                    if comma.fmt(fmt)? {
                        fmt.ws()?;
                    }

                    stack.try_push(UseStep::GroupItem(buf, braced))?;
                    stack.try_push(UseStep::Path(inner.into_stream()))?;
                } else {
                    if !comma.ignore(fmt)? {
                        fmt.comments(Infix)?;
                    }

                    let close = buf.stream().one(K!['}']);

                    if braced {
                        close.fmt(fmt)?;
                    } else {
                        close.ignore(fmt)?;
                    }

                    buf.end()?;
                }
            }
            UseStep::GroupItem(mut buf, braced) => {
                let comma = buf.stream().remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
                stack.try_push(UseStep::Group(buf, comma, braced))?;
            }
        }
    }

    Ok(())
}

/// Emit the brace which opens a group, unless the group holds a single item, in
/// which case it is written without its braces.
///
/// Returns whether the braces are being written, which the brace which closes
/// the group needs to agree on.
fn use_group_open<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>) -> Result<bool> {
    let mut nested = 0;

    for n in p.children() {
        nested += usize::from(matches!(n.kind(), ItemUsePath));

        if nested > 1 {
            break;
        }
    }

    let braced = nested != 1;
    let open = p.expect(K!['{'])?;

    if braced {
        open.fmt(fmt)?;
    } else {
        fmt.ignore(open)?;
    }

    Ok(braced)
}

/// The state of a walk over the tree.
///
/// Expressions, patterns, items and the blocks which contain them all nest
/// through each other. What the formatter emits for a node depends on what its
/// children turned out to be, so rather than recursing it parks the stream it
/// is part way through on `stack`. A deeply nested source therefore costs heap
/// rather than call stack, and no limit has to be imposed on how deeply the
/// input it is handed nests.
struct Cx<'a> {
    stack: Vec<Step<'a>>,
    /// The kind of the expression which was emitted last.
    ///
    /// A match or select arm is written without the comma which separates it
    /// from the next one when its body is a block, which is only known once
    /// that body has been emitted.
    kind: Kind,
}

impl<'a> Cx<'a> {
    #[inline]
    fn push(&mut self, step: Step<'a>) -> Result<()> {
        self.stack.try_push(step)?;
        Ok(())
    }
}

/// A pending step in the walk.
///
/// Each variant which holds a `buf` owns that stream until it is ended, and
/// `at` is how far through emitting the construct the step has got. A step
/// which has to descend pushes itself back at the point it should resume from,
/// then pushes the child it descends into on top of that.
enum Step<'a> {
    /// End a stream which has been emitted in full.
    Done {
        buf: StreamBuf<'a>,
    },
    /// The statements of a block body, or of the root.
    BlockBody {
        buf: StreamBuf<'a>,
        /// Whether the block is written expanded, or `None` at the root, which
        /// has no braces of its own.
        expanded: Option<bool>,
        last: Option<NodeClass>,
        pending: Option<(bool, NodeClass)>,
        at: u8,
    },
    /// A statement, which is a local, an item or an expression.
    Stmt {
        buf: StreamBuf<'a>,
    },
    Local {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// A block, `{ .. }`, together with the braces around its body.
    Block {
        buf: StreamBuf<'a>,
        compact: bool,
        at: u8,
    },
    Item {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// The item itself, once its attributes have been emitted.
    ItemInner {
        buf: StreamBuf<'a>,
    },
    ItemFn {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ItemImpl {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ItemMod {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ItemConst {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ItemStatic {
        buf: StreamBuf<'a>,
        at: u8,
    },
    FnArgs {
        buf: StreamBuf<'a>,
        comma: Remaining<'a>,
        at: u8,
    },
    Pat {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// The pattern itself, once its attributes have been emitted.
    PatInner {
        buf: StreamBuf<'a>,
    },
    /// The elements of an array or of a tuple pattern.
    PatSeq {
        buf: StreamBuf<'a>,
        close: Kind,
        /// Whether the trailing comma of a one element tuple is kept.
        tuple: bool,
        trailing: bool,
        count: usize,
        comma: Remaining<'a>,
        at: u8,
    },
    PatObject {
        buf: StreamBuf<'a>,
        comma: Remaining<'a>,
        at: u8,
    },
    Expr {
        buf: StreamBuf<'a>,
        kind: Kind,
        at: u8,
    },
    /// The expression itself, once its attributes, modifiers and labels have
    /// been emitted.
    InnerExpr {
        buf: StreamBuf<'a>,
    },
    ExprAssign {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// The elements of an array literal, or the arguments of a call.
    Exprs {
        buf: StreamBuf<'a>,
        close: Kind,
        loose: bool,
        comma: Remaining<'a>,
        at: u8,
    },
    ExprTuple {
        buf: StreamBuf<'a>,
        comma: Remaining<'a>,
        count: usize,
        at: u8,
    },
    ExprObject {
        buf: StreamBuf<'a>,
        loose: bool,
        comma: Remaining<'a>,
        at: u8,
    },
    ExprBinary {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// A group, `( .. )`, or the empty group a macro expansion produces.
    Group {
        buf: StreamBuf<'a>,
        paren: bool,
        at: u8,
    },
    ExprIf {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// The `else` or `else if` which follows an `if`.
    ElseBranch {
        buf: StreamBuf<'a>,
        else_if: bool,
        at: u8,
    },
    Condition {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ExprWhile {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ExprFor {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ExprSelect {
        buf: StreamBuf<'a>,
        at: u8,
    },
    SelectArm {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ExprMatch {
        buf: StreamBuf<'a>,
        any: bool,
        at: u8,
    },
    MatchArm {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ExprClosure {
        buf: StreamBuf<'a>,
        at: u8,
    },
    ClosureArgs {
        buf: StreamBuf<'a>,
        comma: Remaining<'a>,
        at: u8,
    },
    ExprChain {
        buf: StreamBuf<'a>,
        /// The element from which the chain is broken over several lines.
        from: usize,
        n: usize,
        unindented: bool,
        at: u8,
    },
    /// One element of the chain applied to an expression.
    ChainLink {
        buf: StreamBuf<'a>,
        at: u8,
    },
    /// The token which separates the ends of a range, and the end which may
    /// follow it.
    RangeTail {
        buf: StreamBuf<'a>,
        more: bool,
    },
}

pub(super) fn root<'a>(fmt: &mut Formatter<'a>, tree: &'a Tree) -> Result<()> {
    tree.parse_all(|p| {
        let mut cx = Cx {
            stack: Vec::new(),
            kind: Eof,
        };

        let buf = p.take_remaining();
        block_body(fmt, &mut cx, buf, None)?;

        while let Some(step) = cx.stack.pop() {
            self::step(fmt, &mut cx, step)?;
        }

        Ok(())
    })?;

    fmt.nl(1)?;
    Ok(())
}

/// Start the body of a block, which the root shares with `{ .. }`.
///
/// `compact` is `None` at the root, which has no braces to write around what it
/// contains.
fn block_body<'a>(
    fmt: &mut Formatter<'a>,
    cx: &mut Cx<'a>,
    mut buf: StreamBuf<'a>,
    compact: Option<bool>,
) -> Result<()> {
    let expanded = match compact {
        Some(compact) => {
            let expanded = !buf.stream().is_eof() || !compact;

            if expanded {
                fmt.indent(1)?;
                fmt.nl(1)?;
                fmt.comments(Line)?;
            } else {
                fmt.comments(Prefix)?;
            }

            Some(expanded)
        }
        None => None,
    };

    inner_attributes(fmt, buf.stream())?;

    cx.push(Step::BlockBody {
        buf,
        expanded,
        last: None,
        pending: None,
        at: 0,
    })
}

/// The step which emits the condition of an `if` or a `while`, which is either
/// a `let` binding or a plain expression.
fn condition_or_expr<'a>(p: &mut Stream<'a>) -> Result<Step<'a>> {
    if let MaybeNode::Some(node) = p.eat(Condition) {
        return Ok(Step::Condition {
            buf: node.into_stream(),
            at: 0,
        });
    }

    let node = p.expect(Expr)?;

    Ok(Step::Expr {
        buf: node.into_stream(),
        kind: Eof,
        at: 0,
    })
}

/// Descend into an expression node.
fn expr<'a>(cx: &mut Cx<'a>, node: Node<'a>) -> Result<()> {
    cx.push(Step::Expr {
        buf: node.into_stream(),
        kind: Eof,
        at: 0,
    })
}

/// Start the elements of an array literal or the arguments of a call, which are
/// laid out over several lines when they do not fit on one.
fn exprs<'a>(
    fmt: &mut Formatter<'a>,
    cx: &mut Cx<'a>,
    mut buf: StreamBuf<'a>,
    open: Kind,
    close: Kind,
) -> Result<()> {
    let loose = {
        let p = buf.stream();

        let mut count = 0;
        let mut expanded = fmt.source.is_at_least(p.span(), 80)?;

        for node in p.children() {
            if expanded {
                break;
            }

            count += usize::from(matches!(node.kind(), Expr));
            expanded |= matches!(node.kind(), Kind::Comment) || count >= 6;
        }

        expanded
    };

    buf.stream().one(open).fmt(fmt)?;

    if loose {
        fmt.nl(1)?;
        fmt.indent(1)?;
    }

    cx.push(Step::Exprs {
        buf,
        close,
        loose,
        comma: Remaining::default(),
        at: 0,
    })
}

/// Perform a single step of the walk.
fn step<'a>(fmt: &mut Formatter<'a>, cx: &mut Cx<'a>, step: Step<'a>) -> Result<()> {
    match step {
        Step::Done { buf } => buf.end(),
        Step::BlockBody {
            mut buf,
            expanded,
            last,
            pending,
            at,
        } => {
            if at == 1 {
                let Some((needs_semi, class)) = pending else {
                    return Err(buf.stream().expected("statement"));
                };

                {
                    let p = buf.stream();
                    let trailing_semi = p.remaining(fmt, K![;])?;

                    if needs_semi || trailing_semi.is_present() {
                        fmt.comments(Suffix)?;
                    }

                    trailing_semi.write_if(fmt, needs_semi)?;
                }

                return cx.push(Step::BlockBody {
                    buf,
                    expanded,
                    last: Some(class),
                    pending: None,
                    at: 0,
                });
            }

            if buf.stream().is_eof() {
                match expanded {
                    Some(true) => {
                        fmt.nl(1)?;
                        fmt.comments(Line)?;
                        fmt.nl(1)?;
                        fmt.indent(-1)?;
                    }
                    Some(false) => {
                        fmt.comments(Suffix)?;
                    }
                    None => {}
                }

                return buf.end();
            }

            let node = buf.stream().pump()?;
            let (needs_semi, class) = classify(&node);

            if let Some(last) = last {
                let n = match last {
                    NodeClass::Item => 1,
                    NodeClass::Const => usize::from(!matches!(class, NodeClass::Const)),
                    NodeClass::Local => usize::from(!matches!(class, NodeClass::Local)),
                    _ => 0,
                };

                fmt.nl(n + 1)?;
            }

            fmt.comments(Line)?;

            cx.push(Step::BlockBody {
                buf,
                expanded,
                last,
                pending: Some((needs_semi, class)),
                at: 1,
            })?;

            cx.push(Step::Stmt {
                buf: node.into_stream(),
            })
        }
        Step::Stmt { mut buf } => match buf.stream().kind() {
            Local => {
                if attributes(fmt, buf.stream())?.skip {
                    buf.stream().write_remaining(fmt)?;
                    return buf.end();
                }

                modifiers(fmt, buf.stream())?;
                cx.push(Step::Local { buf, at: 0 })
            }
            Item => cx.push(Step::Item { buf, at: 0 }),
            _ => cx.push(Step::Expr {
                buf,
                kind: Eof,
                at: 0,
            }),
        },
        Step::Local { mut buf, at } => {
            let node = {
                let p = buf.stream();

                match at {
                    0 => {
                        p.expect(K![let])?.fmt(fmt)?;
                        fmt.ws()?;
                        p.expect(Pat)?
                    }
                    _ => {
                        fmt.ws()?;
                        p.one(K![=]).fmt(fmt)?;
                        fmt.ws()?;
                        p.expect(Expr)?
                    }
                }
            };

            if at == 0 {
                cx.push(Step::Local { buf, at: 1 })?;

                cx.push(Step::Pat {
                    buf: node.into_stream(),
                    at: 0,
                })
            } else {
                cx.push(Step::Done { buf })?;
                self::expr(cx, node)
            }
        }
        Step::Block {
            mut buf,
            compact,
            at,
        } => {
            if at == 1 {
                buf.stream().one(K!['}']).fmt(fmt)?;
                return buf.end();
            }

            let body = {
                let p = buf.stream();
                p.one(K!['{']).fmt(fmt)?;
                p.expect(BlockBody)?
            };

            cx.push(Step::Block {
                buf,
                compact,
                at: 1,
            })?;

            block_body(fmt, cx, body.into_stream(), Some(compact))
        }
        Step::Item { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let attrs = attributes(fmt, buf.stream())?;
            let node = buf.stream().pump()?;

            if attrs.skip {
                node.parse(|p| p.write_remaining(fmt))?;
                return buf.end();
            }

            cx.push(Step::Item { buf, at: 1 })?;

            cx.push(Step::ItemInner {
                buf: node.into_stream(),
            })
        }
        Step::ItemInner { mut buf } => {
            let kind = buf.stream().kind();
            modifiers(fmt, buf.stream())?;

            match kind {
                ItemStruct => {
                    item_struct(fmt, buf.stream())?;
                    buf.end()
                }
                ItemEnum => {
                    item_enum(fmt, buf.stream())?;
                    buf.end()
                }
                ItemUse => {
                    item_use(fmt, buf.stream())?;
                    buf.end()
                }
                ItemFn => cx.push(Step::ItemFn { buf, at: 0 }),
                ItemImpl => cx.push(Step::ItemImpl { buf, at: 0 }),
                ItemMod | ItemFileMod => cx.push(Step::ItemMod { buf, at: 0 }),
                ItemConst => cx.push(Step::ItemConst { buf, at: 0 }),
                ItemStatic => cx.push(Step::ItemStatic { buf, at: 0 }),
                _ => Err(buf.stream().expected(Item)),
            }
        }
        Step::ItemFn { mut buf, at } => match at {
            0 => {
                let args = {
                    let p = buf.stream();
                    p.expect(K![fn])?.fmt(fmt)?;

                    if matches!(p.peek(), K![ident]) {
                        fmt.ws()?;
                        p.pump()?.fmt(fmt)?;
                    }

                    p.eat(FnArgs)
                };

                cx.push(Step::ItemFn { buf, at: 1 })?;

                if let MaybeNode::Some(node) = args {
                    cx.push(Step::FnArgs {
                        buf: node.into_stream(),
                        comma: Remaining::default(),
                        at: 0,
                    })?;
                } else {
                    fmt.lit("()")?;
                }

                Ok(())
            }
            1 => {
                fmt.ws()?;
                let block = buf.stream().eat(Block);

                cx.push(Step::ItemFn { buf, at: 2 })?;

                if let MaybeNode::Some(node) = block {
                    cx.push(Step::Block {
                        buf: node.into_stream(),
                        compact: false,
                        at: 0,
                    })?;
                } else {
                    fmt.lit("{")?;
                    fmt.nl(1)?;
                    fmt.lit("}")?;
                }

                Ok(())
            }
            _ => buf.end(),
        },
        Step::FnArgs {
            mut buf,
            mut comma,
            at,
        } => {
            if at == 0 {
                let p = buf.stream();
                p.expect(K!['('])?.fmt(fmt)?;
                p.remaining(fmt, K![,])?.ignore(fmt)?;
            }

            if at == 2 {
                let p = buf.stream();
                comma = p.remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
            }

            let node = buf.stream().eat(Pat);

            let MaybeNode::Some(node) = node else {
                if !comma.ignore(fmt)? {
                    fmt.comments(Infix)?;
                }

                buf.stream().one(K![')']).fmt(fmt)?;
                return buf.end();
            };

            fmt.comments(Prefix)?;

            if comma.fmt(fmt)? {
                fmt.ws()?;
            }

            cx.push(Step::FnArgs {
                buf,
                comma: Remaining::default(),
                at: 2,
            })?;

            cx.push(Step::Pat {
                buf: node.into_stream(),
                at: 0,
            })
        }
        Step::ItemImpl { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let block = {
                let p = buf.stream();
                p.expect(K![impl])?.fmt(fmt)?;
                fmt.ws()?;
                p.expect(Path)?.parse(|p| path(fmt, p))?;
                fmt.ws()?;
                p.expect(Block)?
            };

            cx.push(Step::ItemImpl { buf, at: 1 })?;

            cx.push(Step::Block {
                buf: block.into_stream(),
                compact: false,
                at: 0,
            })
        }
        Step::ItemMod { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let block = {
                let p = buf.stream();
                p.expect(K![mod])?.fmt(fmt)?;
                fmt.ws()?;
                p.pump()?.fmt(fmt)?;
                p.eat(Block)
            };

            cx.push(Step::ItemMod { buf, at: 1 })?;

            if let MaybeNode::Some(node) = block {
                fmt.ws()?;

                cx.push(Step::Block {
                    buf: node.into_stream(),
                    compact: false,
                    at: 0,
                })?;
            }

            Ok(())
        }
        Step::ItemConst { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let node = {
                let p = buf.stream();
                p.pump()?.fmt(fmt)?;
                fmt.ws()?;
                p.one(K![=]).fmt(fmt)?;
                fmt.ws()?;
                p.pump()?
            };

            cx.push(Step::ItemConst { buf, at: 1 })?;
            self::expr(cx, node)
        }
        Step::ItemStatic { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let node = {
                let p = buf.stream();
                p.pump()?.fmt(fmt)?;

                if let MaybeNode::Some(eq) = p.eat(K![=]) {
                    fmt.ws()?;
                    eq.fmt(fmt)?;
                    fmt.ws()?;
                    Some(p.pump()?)
                } else {
                    None
                }
            };

            cx.push(Step::ItemStatic { buf, at: 1 })?;

            if let Some(node) = node {
                return self::expr(cx, node);
            }

            Ok(())
        }
        Step::Pat { mut buf, at } => {
            if at == 1 {
                return buf.end();
            }

            let node = {
                let p = buf.stream();

                while let MaybeNode::Some(attr) = p.eat(Attribute) {
                    attr.fmt(fmt)?;
                    fmt.ws()?;
                }

                p.pump()?
            };

            cx.push(Step::Pat { buf, at: 1 })?;

            cx.push(Step::PatInner {
                buf: node.into_stream(),
            })
        }
        Step::PatInner { mut buf } => {
            let kind = buf.stream().kind();

            match kind {
                Lit => {
                    let p = buf.stream();
                    p.eat(K![-]).fmt(fmt)?;
                    p.pump()?.fmt(fmt)?;
                    buf.end()
                }
                PatIgnore | K![..] => {
                    buf.stream().pump()?.fmt(fmt)?;
                    buf.end()
                }
                Path => {
                    path(fmt, buf.stream())?;
                    buf.end()
                }
                PatArray => {
                    buf.stream().expect(K!['['])?.fmt(fmt)?;

                    cx.push(Step::PatSeq {
                        buf,
                        close: K![']'],
                        tuple: false,
                        trailing: false,
                        count: 0,
                        comma: Remaining::default(),
                        at: 0,
                    })
                }
                PatTuple => {
                    let trailing = {
                        let p = buf.stream();
                        let trailing = p.eat(Path).parse(|p| path(fmt, p))?.is_none();
                        p.expect(K!['('])?.fmt(fmt)?;
                        trailing
                    };

                    cx.push(Step::PatSeq {
                        buf,
                        close: K![')'],
                        tuple: true,
                        trailing,
                        count: 0,
                        comma: Remaining::default(),
                        at: 0,
                    })
                }
                PatObject => {
                    {
                        let p = buf.stream();

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
                    }

                    cx.push(Step::PatObject {
                        buf,
                        comma: Remaining::default(),
                        at: 0,
                    })
                }
                _ => Err(buf.stream().expected("pattern")),
            }
        }
        Step::PatSeq {
            mut buf,
            close,
            tuple,
            trailing,
            mut count,
            mut comma,
            at,
        } => {
            if at == 1 {
                let p = buf.stream();
                comma = p.remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
            }

            let node = buf.stream().eat_matching(|k| matches!(k, Pat | K![..]));

            let MaybeNode::Some(node) = node else {
                if tuple {
                    if count == 1 && trailing {
                        comma.fmt(fmt)?;
                    } else {
                        comma.ignore(fmt)?;

                        if count == 0 {
                            fmt.comments(Infix)?;
                        }
                    }
                } else if !comma.ignore(fmt)? {
                    fmt.comments(Infix)?;
                }

                buf.stream().one(close).fmt(fmt)?;
                return buf.end();
            };

            fmt.comments(Prefix)?;

            if comma.fmt(fmt)? {
                fmt.ws()?;
            }

            count += 1;

            if matches!(node.kind(), K![..]) {
                node.fmt(fmt)?;

                return cx.push(Step::PatSeq {
                    buf,
                    close,
                    tuple,
                    trailing,
                    count,
                    comma: Remaining::default(),
                    at: 1,
                });
            }

            cx.push(Step::PatSeq {
                buf,
                close,
                tuple,
                trailing,
                count,
                comma: Remaining::default(),
                at: 1,
            })?;

            cx.push(Step::Pat {
                buf: node.into_stream(),
                at: 0,
            })
        }
        Step::PatObject {
            mut buf,
            mut comma,
            at,
        } => {
            if at == 1 {
                comma = buf.stream().remaining(fmt, K![,])?;
            }

            if !matches!(buf.stream().peek(), object_key!() | K![..]) {
                if comma.ignore(fmt)? {
                    fmt.ws()?;
                } else {
                    fmt.comments(Infix)?;
                }

                buf.stream().remaining(fmt, K!['}'])?.fmt(fmt)?;
                return buf.end();
            }

            comma.fmt(fmt)?;
            fmt.ws()?;

            let node = {
                let p = buf.stream();

                match p.peek() {
                    object_key!() => {
                        p.pump()?.fmt(fmt)?;

                        if let MaybeNode::Some(colon) = p.eat(K![:]) {
                            colon.fmt(fmt)?;
                            fmt.ws()?;
                            Some(p.expect(Pat)?)
                        } else {
                            None
                        }
                    }
                    _ => {
                        p.expect(K![..])?.fmt(fmt)?;
                        None
                    }
                }
            };

            cx.push(Step::PatObject {
                buf,
                comma: Remaining::default(),
                at: 1,
            })?;

            if let Some(node) = node {
                return cx.push(Step::Pat {
                    buf: node.into_stream(),
                    at: 0,
                });
            }

            Ok(())
        }
        Step::Expr { mut buf, kind, at } => {
            if at == 1 {
                cx.kind = kind;
                return buf.end();
            }

            let mut skip = false;

            {
                let p = buf.stream();

                while let MaybeNode::Some(attr) = p.eat(Attribute) {
                    skip |= is_runefmt_skip(fmt, attr.clone());
                    attr.fmt(fmt)?;
                    fmt.ws()?;
                }
            }

            if skip {
                buf.stream().write_remaining(fmt)?;
                cx.kind = Expr;
                return buf.end();
            }

            modifiers(fmt, buf.stream())?;
            expr_labels(fmt, buf.stream())?;

            let node = buf.stream().pump()?;

            cx.push(Step::Expr {
                buf,
                kind: node.kind(),
                at: 1,
            })?;

            cx.push(Step::InnerExpr {
                buf: node.into_stream(),
            })
        }
        Step::InnerExpr { mut buf } => {
            let kind = buf.stream().kind();

            match kind {
                Path => {
                    path(fmt, buf.stream())?;
                    buf.end()
                }
                Lit => {
                    let p = buf.stream();
                    p.eat(K![-]).fmt(fmt)?;
                    p.pump()?.fmt(fmt)?;
                    buf.end()
                }
                TemplateString | ExprRangeFull => {
                    buf.stream().pump()?.fmt(fmt)?;
                    buf.end()
                }
                Block => cx.push(Step::Block {
                    buf,
                    compact: true,
                    at: 0,
                }),
                ExprAssign => cx.push(Step::ExprAssign { buf, at: 0 }),
                ExprArray => exprs(fmt, cx, buf, K!['['], K![']']),
                ExprTuple => {
                    buf.stream().expect(K!['('])?.fmt(fmt)?;

                    cx.push(Step::ExprTuple {
                        buf,
                        comma: Remaining::default(),
                        count: 0,
                        at: 0,
                    })
                }
                ExprObject => {
                    let loose = {
                        let p = buf.stream();

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

                        p.expect(K!['{'])?.fmt(fmt)?;
                        expanded
                    };

                    if loose {
                        fmt.nl(1)?;
                        fmt.indent(1)?;
                    }

                    cx.push(Step::ExprObject {
                        buf,
                        loose,
                        comma: Remaining::default(),
                        at: 0,
                    })
                }
                ExprBinary => cx.push(Step::ExprBinary { buf, at: 0 }),
                ExprUnary => {
                    let node = {
                        let p = buf.stream();
                        p.pump()?.fmt(fmt)?;
                        p.pump()?
                    };

                    cx.push(Step::Done { buf })?;

                    cx.push(Step::InnerExpr {
                        buf: node.into_stream(),
                    })
                }
                ExprGroup => {
                    buf.stream().expect(K!['('])?.fmt(fmt)?;

                    cx.push(Step::Group {
                        buf,
                        paren: true,
                        at: 0,
                    })
                }
                ExprEmptyGroup => {
                    buf.stream()
                        .expect(Kind::Open(Delimiter::Empty))?
                        .ignore(fmt)?;

                    cx.push(Step::Group {
                        buf,
                        paren: false,
                        at: 0,
                    })
                }
                ExprIf => cx.push(Step::ExprIf { buf, at: 0 }),
                ExprWhile => cx.push(Step::ExprWhile { buf, at: 0 }),
                ExprLoop => {
                    let block = {
                        let p = buf.stream();
                        p.expect(K![loop])?.fmt(fmt)?;
                        fmt.ws()?;
                        p.expect(Block)?
                    };

                    cx.push(Step::Done { buf })?;

                    cx.push(Step::Block {
                        buf: block.into_stream(),
                        compact: false,
                        at: 0,
                    })
                }
                ExprBreak | ExprReturn | ExprYield => {
                    let node = {
                        let p = buf.stream();

                        let keyword = match kind {
                            ExprBreak => K![break],
                            ExprReturn => K![return],
                            _ => K![yield],
                        };

                        p.expect(keyword)?.fmt(fmt)?;

                        if matches!(kind, ExprBreak) {
                            while matches!(p.peek(), K!['label]) {
                                fmt.ws()?;
                                p.pump()?.fmt(fmt)?;
                            }
                        }

                        p.eat(Expr)
                    };

                    let MaybeNode::Some(node) = node else {
                        return buf.end();
                    };

                    fmt.ws()?;
                    cx.push(Step::Done { buf })?;
                    self::expr(cx, node)
                }
                ExprContinue => {
                    expr_continue(fmt, buf.stream())?;
                    buf.end()
                }
                ExprFor => cx.push(Step::ExprFor { buf, at: 0 }),
                ExprMatch => cx.push(Step::ExprMatch {
                    buf,
                    any: false,
                    at: 0,
                }),
                ExprSelect => cx.push(Step::ExprSelect { buf, at: 0 }),
                ExprRangeFrom => {
                    let node = buf.stream().pump()?;

                    cx.push(Step::RangeTail { buf, more: false })?;

                    cx.push(Step::InnerExpr {
                        buf: node.into_stream(),
                    })
                }
                ExprRangeTo | ExprRangeToInclusive => {
                    let node = {
                        let p = buf.stream();
                        p.pump()?.fmt(fmt)?;
                        p.pump()?
                    };

                    cx.push(Step::Done { buf })?;

                    cx.push(Step::InnerExpr {
                        buf: node.into_stream(),
                    })
                }
                ExprRange | ExprRangeInclusive => {
                    let node = buf.stream().pump()?;

                    cx.push(Step::RangeTail { buf, more: true })?;

                    cx.push(Step::InnerExpr {
                        buf: node.into_stream(),
                    })
                }
                ExprClosure => cx.push(Step::ExprClosure { buf, at: 0 }),
                ExprChain => cx.push(Step::ExprChain {
                    buf,
                    from: 0,
                    n: 0,
                    unindented: true,
                    at: 0,
                }),
                ExprMacroCall => {
                    let p = buf.stream();
                    p.expect(Path)?.parse(|p| path(fmt, p))?;
                    p.expect(K![!])?.fmt(fmt)?;

                    match p.peek() {
                        K!['{'] => loose_expr_macro_call(fmt, p)?,
                        K!['['] => compact_expr_macro_call(fmt, p, K!['['], K![']'])?,
                        _ => compact_expr_macro_call(fmt, p, K!['('], K![')'])?,
                    }

                    buf.end()
                }
                Error if fmt.options.error_recovery => {
                    buf.stream().fmt_remaining_trimmed(fmt)?;
                    buf.end()
                }
                _ => Err(buf.stream().expected("inner expression")),
            }
        }
        Step::RangeTail { mut buf, more } => {
            let node = {
                let p = buf.stream();
                p.pump()?.fmt(fmt)?;

                if !more {
                    None
                } else {
                    Some(p.pump()?)
                }
            };

            let Some(node) = node else {
                return buf.end();
            };

            cx.push(Step::Done { buf })?;

            cx.push(Step::InnerExpr {
                buf: node.into_stream(),
            })
        }
        Step::ExprAssign { mut buf, at } => {
            let node = {
                let p = buf.stream();

                if at == 1 {
                    fmt.ws()?;
                    p.expect(K![=])?.fmt(fmt)?;
                    fmt.ws()?;
                }

                p.expect(Expr)?
            };

            if at == 0 {
                cx.push(Step::ExprAssign { buf, at: 1 })?;
            } else {
                cx.push(Step::Done { buf })?;
            }

            self::expr(cx, node)
        }
        Step::Exprs {
            mut buf,
            close,
            loose,
            mut comma,
            at,
        } => {
            if at == 1 {
                let p = buf.stream();

                if loose {
                    p.remaining(fmt, K![,])?.fmt(fmt)?;
                    fmt.nl(1)?;
                } else {
                    comma = p.remaining(fmt, K![,])?;
                    fmt.comments(Suffix)?;
                }
            }

            let node = buf.stream().eat(Expr);

            let MaybeNode::Some(node) = node else {
                if loose {
                    fmt.nl(1)?;
                    fmt.comments(Line)?;
                    fmt.indent(-1)?;
                } else if !comma.ignore(fmt)? {
                    fmt.comments(Infix)?;
                }

                buf.stream().one(close).fmt(fmt)?;
                return buf.end();
            };

            if loose {
                fmt.comments(Line)?;
            } else {
                fmt.comments(Prefix)?;

                if comma.fmt(fmt)? {
                    fmt.ws()?;
                }
            }

            cx.push(Step::Exprs {
                buf,
                close,
                loose,
                comma: Remaining::default(),
                at: 1,
            })?;

            self::expr(cx, node)
        }
        Step::ExprTuple {
            mut buf,
            mut comma,
            mut count,
            at,
        } => {
            if at == 1 {
                let p = buf.stream();
                comma = p.remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
            }

            let node = buf.stream().eat(Expr);

            let MaybeNode::Some(node) = node else {
                if count == 1 {
                    comma.fmt(fmt)?;
                } else {
                    comma.ignore(fmt)?;

                    if count == 0 {
                        fmt.comments(Infix)?;
                    }
                }

                buf.stream().one(K![')']).fmt(fmt)?;
                return buf.end();
            };

            fmt.comments(Prefix)?;

            if comma.fmt(fmt)? {
                fmt.ws()?;
            }

            count += 1;

            cx.push(Step::ExprTuple {
                buf,
                comma: Remaining::default(),
                count,
                at: 1,
            })?;

            self::expr(cx, node)
        }
        Step::ExprObject {
            mut buf,
            loose,
            mut comma,
            at,
        } => {
            if at == 1 {
                let p = buf.stream();

                if loose {
                    p.remaining(fmt, K![,])?.fmt(fmt)?;
                    fmt.nl(1)?;
                } else {
                    comma = p.remaining(fmt, K![,])?;
                }
            }

            if !matches!(buf.stream().peek(), object_key!()) {
                if loose {
                    fmt.nl(1)?;
                    fmt.indent(-1)?;
                } else if comma.ignore(fmt)? {
                    fmt.ws()?;
                } else {
                    fmt.comments(Infix)?;
                }

                buf.stream().remaining(fmt, K!['}'])?.fmt(fmt)?;
                return buf.end();
            }

            let node = {
                let p = buf.stream();

                if loose {
                    fmt.comments(Line)?;
                } else {
                    comma.fmt(fmt)?;
                    fmt.ws()?;
                }

                p.pump()?.fmt(fmt)?;

                if let MaybeNode::Some(colon) = p.eat(K![:]) {
                    colon.fmt(fmt)?;
                    fmt.ws()?;
                    Some(p.pump()?)
                } else {
                    None
                }
            };

            cx.push(Step::ExprObject {
                buf,
                loose,
                comma: Remaining::default(),
                at: 1,
            })?;

            if let Some(node) = node {
                return self::expr(cx, node);
            }

            Ok(())
        }
        Step::ExprBinary { mut buf, at } => {
            let node = {
                let p = buf.stream();

                if at == 0 {
                    Some(p.pump()?)
                } else if let MaybeNode::Some(op) = p.eat(ExprOperator) {
                    fmt.ws()?;
                    op.fmt(fmt)?;
                    fmt.ws()?;
                    Some(p.pump()?)
                } else {
                    None
                }
            };

            let Some(node) = node else {
                return buf.end();
            };

            cx.push(Step::ExprBinary { buf, at: 1 })?;

            cx.push(Step::InnerExpr {
                buf: node.into_stream(),
            })
        }
        Step::Group { mut buf, paren, at } => {
            if at == 1 {
                fmt.comments(Suffix)?;
                close_group(fmt, buf.stream(), paren)?;
                return buf.end();
            }

            let node = buf.stream().eat(Expr);

            let MaybeNode::Some(node) = node else {
                fmt.comments(Infix)?;
                close_group(fmt, buf.stream(), paren)?;
                return buf.end();
            };

            fmt.comments(Prefix)?;

            cx.push(Step::Group { buf, paren, at: 1 })?;

            self::expr(cx, node)
        }
        Step::ExprIf { mut buf, at } => match at {
            0 => {
                let child = {
                    let p = buf.stream();
                    p.expect(If)?.fmt(fmt)?;
                    fmt.ws()?;
                    condition_or_expr(p)?
                };

                cx.push(Step::ExprIf { buf, at: 1 })?;
                cx.push(child)
            }
            1 => {
                fmt.ws()?;
                let block = buf.stream().eat(Block);

                cx.push(Step::ExprIf { buf, at: 2 })?;

                if let MaybeNode::Some(node) = block {
                    cx.push(Step::Block {
                        buf: node.into_stream(),
                        compact: false,
                        at: 0,
                    })?;
                } else {
                    fmt.lit("{}")?;
                }

                Ok(())
            }
            _ => {
                let Some(node) = buf.stream().next() else {
                    return buf.end();
                };

                cx.push(Step::ExprIf { buf, at: 2 })?;

                match node.kind() {
                    ExprElse => cx.push(Step::ElseBranch {
                        buf: node.into_stream(),
                        else_if: false,
                        at: 0,
                    }),
                    ExprElseIf => cx.push(Step::ElseBranch {
                        buf: node.into_stream(),
                        else_if: true,
                        at: 0,
                    }),
                    _ => {
                        node.fmt(fmt)?;
                        Ok(())
                    }
                }
            }
        },
        Step::ElseBranch {
            mut buf,
            else_if,
            at,
        } => {
            if at == 0 {
                let child = {
                    let p = buf.stream();
                    fmt.ws()?;
                    p.expect(K![else])?.fmt(fmt)?;
                    fmt.ws()?;

                    if else_if {
                        p.expect(K![if])?.fmt(fmt)?;
                        fmt.ws()?;
                        Some(condition_or_expr(p)?)
                    } else {
                        None
                    }
                };

                if let Some(child) = child {
                    cx.push(Step::ElseBranch {
                        buf,
                        else_if,
                        at: 1,
                    })?;

                    return cx.push(child);
                }
            } else {
                fmt.ws()?;
            }

            let block = buf.stream().expect(Block)?;
            cx.push(Step::Done { buf })?;

            cx.push(Step::Block {
                buf: block.into_stream(),
                compact: false,
                at: 0,
            })
        }
        Step::Condition { mut buf, at } => {
            let node = {
                let p = buf.stream();

                if at == 0 {
                    p.expect(K![let])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(Pat)?
                } else {
                    fmt.ws()?;
                    p.expect(K![=])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(Expr)?
                }
            };

            if at == 0 {
                cx.push(Step::Condition { buf, at: 1 })?;

                return cx.push(Step::Pat {
                    buf: node.into_stream(),
                    at: 0,
                });
            }

            cx.push(Step::Done { buf })?;
            self::expr(cx, node)
        }
        Step::ExprWhile { mut buf, at } => {
            if at == 0 {
                let child = {
                    let p = buf.stream();
                    p.expect(K![while])?.fmt(fmt)?;
                    fmt.ws()?;
                    condition_or_expr(p)?
                };

                cx.push(Step::ExprWhile { buf, at: 1 })?;
                return cx.push(child);
            }

            fmt.ws()?;
            let block = buf.stream().expect(Block)?;
            cx.push(Step::Done { buf })?;

            cx.push(Step::Block {
                buf: block.into_stream(),
                compact: false,
                at: 0,
            })
        }
        Step::ExprFor { mut buf, at } => match at {
            0 => {
                let node = {
                    let p = buf.stream();
                    p.expect(K![for])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.expect(Pat)?
                };

                cx.push(Step::ExprFor { buf, at: 1 })?;

                cx.push(Step::Pat {
                    buf: node.into_stream(),
                    at: 0,
                })
            }
            1 => {
                let node = {
                    let p = buf.stream();
                    fmt.ws()?;
                    p.expect(K![in])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.pump()?
                };

                cx.push(Step::ExprFor { buf, at: 2 })?;
                self::expr(cx, node)
            }
            _ => {
                fmt.ws()?;
                let block = buf.stream().expect(Block)?;
                cx.push(Step::Done { buf })?;

                cx.push(Step::Block {
                    buf: block.into_stream(),
                    compact: false,
                    at: 0,
                })
            }
        },
        Step::ExprSelect { mut buf, at } => {
            if at == 0 {
                let p = buf.stream();
                p.expect(K![select])?.fmt(fmt)?;
                fmt.ws()?;

                let MaybeNode::Some(open) = p.eat(K!['{']) else {
                    fmt.lit("{}")?;
                    return buf.end();
                };

                fmt.indent(1)?;
                open.fmt(fmt)?;
            }

            if at == 2 {
                buf.stream()
                    .remaining(fmt, K![,])?
                    .write_only_if(fmt, !matches!(cx.kind, Block))?;
            }

            let node = buf.stream().eat(ExprSelectArm);

            let MaybeNode::Some(node) = node else {
                fmt.comments(Line)?;
                fmt.nl(1)?;
                fmt.indent(-1)?;
                buf.stream().one(K!['}']).fmt(fmt)?;
                return buf.end();
            };

            fmt.nl(1)?;
            fmt.comments(Line)?;

            cx.push(Step::ExprSelect { buf, at: 2 })?;

            cx.push(Step::SelectArm {
                buf: node.into_stream(),
                at: 0,
            })
        }
        Step::SelectArm { mut buf, at } => match at {
            0 => {
                let node = {
                    let p = buf.stream();

                    match p.peek() {
                        K![default] => {
                            p.expect(K![default])?.fmt(fmt)?;
                            None
                        }
                        _ => Some(p.expect(Pat)?),
                    }
                };

                cx.push(Step::SelectArm { buf, at: 1 })?;

                if let Some(node) = node {
                    return cx.push(Step::Pat {
                        buf: node.into_stream(),
                        at: 0,
                    });
                }

                Ok(())
            }
            1 => {
                let node = {
                    let p = buf.stream();

                    if let MaybeNode::Some(eq) = p.eat(K![=]) {
                        fmt.ws()?;
                        eq.fmt(fmt)?;
                        fmt.ws()?;
                        Some(p.pump()?)
                    } else {
                        None
                    }
                };

                cx.push(Step::SelectArm { buf, at: 2 })?;

                if let Some(node) = node {
                    return self::expr(cx, node);
                }

                Ok(())
            }
            _ => {
                let node = {
                    let p = buf.stream();
                    fmt.ws()?;
                    p.one(K![=>]).fmt(fmt)?;
                    fmt.ws()?;
                    p.pump()?
                };

                cx.push(Step::Done { buf })?;
                self::expr(cx, node)
            }
        },
        Step::ExprMatch {
            mut buf,
            mut any,
            at,
        } => match at {
            0 => {
                let node = {
                    let p = buf.stream();
                    p.expect(K![match])?.fmt(fmt)?;
                    fmt.ws()?;
                    p.pump()?
                };

                cx.push(Step::ExprMatch { buf, any, at: 1 })?;
                self::expr(cx, node)
            }
            1 => {
                fmt.ws()?;
                buf.stream().one(K!['{']).fmt(fmt)?;
                cx.push(Step::ExprMatch { buf, any, at: 2 })
            }
            _ => {
                if at == 3 {
                    buf.stream()
                        .remaining(fmt, K![,])?
                        .write_only_if(fmt, !matches!(cx.kind, Block))?;
                }

                let node = buf.stream().eat(ExprMatchArm);

                let MaybeNode::Some(node) = node else {
                    if any {
                        fmt.comments(Line)?;
                        fmt.nl(1)?;
                        fmt.indent(-1)?;
                    } else {
                        fmt.comments(Infix)?;
                    }

                    buf.stream().one(K!['}']).fmt(fmt)?;
                    return buf.end();
                };

                if !any {
                    fmt.indent(1)?;
                }

                any = true;

                cx.push(Step::ExprMatch { buf, any, at: 3 })?;

                cx.push(Step::MatchArm {
                    buf: node.into_stream(),
                    at: 0,
                })
            }
        },
        Step::MatchArm { mut buf, at } => match at {
            0 => {
                let node = {
                    let p = buf.stream();
                    fmt.nl(1)?;
                    fmt.comments(Line)?;
                    p.expect(Pat)?
                };

                cx.push(Step::MatchArm { buf, at: 1 })?;

                cx.push(Step::Pat {
                    buf: node.into_stream(),
                    at: 0,
                })
            }
            1 => {
                let node = {
                    let p = buf.stream();

                    if let MaybeNode::Some(node) = p.eat(K![if]) {
                        fmt.ws()?;
                        node.fmt(fmt)?;
                        fmt.ws()?;
                        Some(p.expect(Expr)?)
                    } else {
                        None
                    }
                };

                cx.push(Step::MatchArm { buf, at: 2 })?;

                if let Some(node) = node {
                    return self::expr(cx, node);
                }

                Ok(())
            }
            _ => {
                let node = {
                    let p = buf.stream();
                    fmt.ws()?;
                    p.one(K![=>]).fmt(fmt)?;
                    fmt.ws()?;
                    p.pump()?
                };

                cx.push(Step::Done { buf })?;
                self::expr(cx, node)
            }
        },
        Step::ExprClosure { mut buf, at } => {
            if at == 0 {
                let args = buf.stream().eat(ClosureArguments);

                cx.push(Step::ExprClosure { buf, at: 1 })?;

                if let MaybeNode::Some(node) = args {
                    return cx.push(Step::ClosureArgs {
                        buf: node.into_stream(),
                        comma: Remaining::default(),
                        at: 0,
                    });
                }

                fmt.lit("||")?;
                return Ok(());
            }

            fmt.ws()?;
            let node = buf.stream().eat(Expr);

            let MaybeNode::Some(node) = node else {
                fmt.lit("{}")?;
                return buf.end();
            };

            cx.push(Step::Done { buf })?;
            self::expr(cx, node)
        }
        Step::ClosureArgs {
            mut buf,
            mut comma,
            at,
        } => {
            if at == 0 {
                let p = buf.stream();

                if let MaybeNode::Some(open) = p.eat(K![||]) {
                    open.fmt(fmt)?;
                    return buf.end();
                }

                p.expect(K![|])?.fmt(fmt)?;
            }

            if at == 2 {
                let p = buf.stream();
                comma = p.remaining(fmt, K![,])?;
                fmt.comments(Suffix)?;
            }

            let node = buf.stream().eat(Pat);

            let MaybeNode::Some(node) = node else {
                if !comma.ignore(fmt)? {
                    fmt.comments(Infix)?;
                }

                buf.stream().one(K![|]).fmt(fmt)?;
                return buf.end();
            };

            fmt.comments(Prefix)?;

            if comma.fmt(fmt)? {
                fmt.ws()?;
            }

            cx.push(Step::ClosureArgs {
                buf,
                comma: Remaining::default(),
                at: 2,
            })?;

            cx.push(Step::Pat {
                buf: node.into_stream(),
                at: 0,
            })
        }
        Step::ExprChain {
            mut buf,
            mut from,
            mut n,
            mut unindented,
            at,
        } => {
            if at == 0 {
                let node = {
                    let p = buf.stream();
                    let expanded = fmt.source.is_at_least(p.span(), 80)?;
                    let node = p.pump()?;
                    let head = node.span();

                    // If the first expression *is* small, and there are no
                    // other expressions that need indentation in the chain, we
                    // can keep it all on one line.
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

                    node
                };

                cx.push(Step::ExprChain {
                    buf,
                    from,
                    n: 0,
                    unindented: true,
                    at: 1,
                })?;

                return cx.push(Step::InnerExpr {
                    buf: node.into_stream(),
                });
            }

            let Some(node) = buf.stream().next() else {
                if !unindented {
                    fmt.indent(-1)?;
                }

                return buf.end();
            };

            if n >= from {
                let first = unindented;
                unindented = false;
                fmt.indent(isize::from(first))?;
                fmt.nl(usize::from(matches!(node.kind(), ExprField | ExprAwait)))?;
            }

            n += 1;

            cx.push(Step::ExprChain {
                buf,
                from,
                n,
                unindented,
                at: 1,
            })?;

            cx.push(Step::ChainLink {
                buf: node.into_stream(),
                at: 0,
            })
        }
        Step::ChainLink { mut buf, at } => {
            if at == 1 {
                let p = buf.stream();
                fmt.comments(Suffix)?;
                p.one(K![']']).fmt(fmt)?;
                return buf.end();
            }

            let kind = buf.stream().kind();

            match kind {
                ExprTry => {
                    buf.stream().one(K![?]).fmt(fmt)?;
                    buf.end()
                }
                ExprAwait => {
                    let p = buf.stream();
                    p.one(K![.]).fmt(fmt)?;
                    p.one(K![await]).fmt(fmt)?;
                    buf.end()
                }
                ExprField => {
                    let p = buf.stream();
                    p.one(K![.]).fmt(fmt)?;

                    match p.peek() {
                        K![number] => {
                            p.pump()?.fmt(fmt)?;
                        }
                        _ => {
                            p.expect(Path)?.parse(|p| path(fmt, p))?;
                        }
                    }

                    buf.end()
                }
                ExprCall => exprs(fmt, cx, buf, K!['('], K![')']),
                ExprIndex => {
                    let node = {
                        let p = buf.stream();
                        p.expect(K!['['])?.fmt(fmt)?;
                        fmt.comments(Prefix)?;
                        p.pump()?
                    };

                    cx.push(Step::ChainLink { buf, at: 1 })?;
                    self::expr(cx, node)
                }
                _ => Err(buf.stream().expected(ExprChain)),
            }
        }
    }
}

/// Emit the token which closes a group.
///
/// The empty group a macro expansion produces is closed by the same token which
/// opened it, and neither is written.
fn close_group<'a>(fmt: &mut Formatter<'a>, p: &mut Stream<'a>, paren: bool) -> Result<()> {
    if paren {
        p.one(K![')']).fmt(fmt)?;
    } else {
        p.one(Kind::Open(Delimiter::Empty)).ignore(fmt)?;
    }

    Ok(())
}
