use core::mem::replace;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::{self, Kind, Span, Spanned};
use crate::compile::{
    meta, Doc, DynLocation, Error, ErrorKind, Location, Result, Visibility, WithSpan,
};
use crate::grammar::{Ignore, MaybeNode, Node, NodeId, Remaining, Stream, StreamBuf};
use crate::indexing;
use crate::parse::Resolve;
use crate::query::{Attrs, BuiltInLiteral, DeferEntry, ExpandMacroBuiltin, ImplItem, ImplItemKind};
use crate::runtime::Call;
use crate::worker::{self, Import, ImportKind, ImportState};

use super::items::Guard;
use super::{validate_call, IndexItem, Indexed, Indexer};

use Kind::*;

/// The kind of an [ExprBlock].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprBlockKind {
    Default,
    Async,
    Const,
}

#[derive(Default, Debug)]
enum ExprSupport {
    Yes,
    #[default]
    No,
}

struct Function<'a> {
    node: Node<'a>,
    name: Option<ast::Ident>,
    attrs: Attrs,
    mods: Mods,
}

fn is_instance(node: &Node<'_>) -> alloc::Result<(bool, Vec<Span>)> {
    fn is_self(node: Node<'_>) -> bool {
        let Some([node]) = node.nodes::<1>() else {
            return false;
        };

        matches!(
            (node.kind(), node.kinds()),
            (IndexedPath(..), Some([K![self]]))
        )
    }

    let mut is_instance = false;
    let mut args = Vec::new();

    let Some(node) = node.find(FnArgs) else {
        return Ok((is_instance, args));
    };

    for node in node.children() {
        if matches!(node.kind(), Pat) {
            args.try_push(node.span())?;
            is_instance |= is_self(node);
        }
    }

    Ok((is_instance, args))
}

/// Indexing event.
#[derive(Debug)]
enum State<'a> {
    Stream(StreamBuf<'a>),
    Expr(Mods, Attrs),
    Block(Guard, IndexItem),
    ConstBlock(Guard, IndexItem, Node<'a>),
    AsyncBlock(Guard, IndexItem, Span, Option<ast::Move>),
    Bare(IndexItem, NodeId),
    Function(Guard, IndexItem, usize, Option<Span>, ExprSupport),
    Module(Guard, IndexItem),
    Closure(Guard, IndexItem, Option<ast::Async>, Option<ast::Move>),
    Const(Guard, IndexItem, Option<Span>, Node<'a>, ExprSupport),
}

struct Processor<'a> {
    span: Span,
    fns: Vec<Function<'a>>,
    stack: Vec<State<'a>>,
    expr: ExprSupport,
}

impl<'a> Processor<'a> {
    fn new(span: Span) -> Self {
        Self {
            span,
            fns: Vec::new(),
            stack: Vec::new(),
            expr: ExprSupport::No,
        }
    }

    fn with_expr(mut self, expr: ExprSupport) -> Self {
        self.expr = expr;
        self
    }

    fn process(mut self, idx: &mut Indexer<'_, '_>) -> Result<()> {
        loop {
            if let Some(State::Stream(p)) = self.stack.last_mut() {
                let Some(node) = p.next() else {
                    self.stack.pop();
                    continue;
                };

                tracing::trace!(?node, stack = self.stack.len());
                self.node(idx, node)?;
                continue;
            };

            let Some(state) = self.stack.pop() else {
                break;
            };

            tracing::trace!(?state, stack = self.stack.len());

            match state {
                State::Expr(mods, attrs) => {
                    mods.deny_all(idx)?;
                    attrs.deny_any(idx)?;
                }
                State::Block(guard, item) => {
                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                }
                State::AsyncBlock(guard, item, span, move_token) => {
                    let item_meta = idx
                        .q
                        .item_for("async block", idx.item.id)
                        .with_span(self.span)?;
                    let layer = idx.scopes.pop().with_span(self.span)?;
                    let call = validate_call(false, true, &layer)?;

                    if let Some(call) = call {
                        idx.q.index_meta(
                            &span,
                            item_meta,
                            meta::Kind::AsyncBlock {
                                call,
                                do_move: move_token.is_some(),
                            },
                        )?;
                    } else {
                        idx.error(Error::new(span, ErrorKind::ClosureKind))?;
                    }

                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                }
                State::ConstBlock(guard, item, node) => {
                    let item_meta = idx
                        .q
                        .item_for("const block", idx.item.id)
                        .with_span(self.span)?;

                    idx.q.index_const_block(
                        item_meta,
                        indexing::ConstBlock::Node(node.node_at(idx.source_id, idx.tree.clone())),
                    )?;

                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                }
                State::Closure(guard, item, async_token, move_token) => {
                    let layer = idx.scopes.pop().with_span(self.span)?;
                    let call = validate_call(false, async_token.is_some(), &layer)?;

                    let Some(call) = call else {
                        return Err(Error::new(self.span, ErrorKind::ClosureKind));
                    };

                    let item_meta = idx
                        .q
                        .item_for("closure", idx.item.id)
                        .with_span(self.span)?;

                    idx.q.index_meta(
                        &self.span,
                        item_meta,
                        meta::Kind::Closure {
                            call,
                            do_move: move_token.is_some(),
                        },
                    )?;

                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                }
                State::Bare(item, id) => {
                    self.bare(idx, id)?;

                    idx.item = item;
                }
                State::Function(guard, item, index, nested_item, expr) => {
                    self.function(idx, index, nested_item)?;

                    self.expr = expr;

                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                    idx.nested_item = nested_item;
                }
                State::Module(guard, item) => {
                    idx.item = item;
                    idx.items.pop(guard).with_span(self.span)?;
                }
                State::Const(guard, idx_item, nested_item, node, expr) => {
                    let item_meta = idx.q.item_for("const", idx.item.id).with_span(self.span)?;

                    idx.q.index_const_expr(
                        item_meta,
                        indexing::ConstExpr::Node(node.node_at(idx.source_id, idx.tree.clone())),
                    )?;

                    self.expr = expr;

                    idx.items.pop(guard).with_span(&node)?;
                    idx.item = idx_item;
                    idx.nested_item = nested_item;
                }
                State::Stream(..) => {}
            }
        }

        Ok(())
    }

    fn node(&mut self, idx: &mut Indexer<'_, '_>, node: Node<'a>) -> Result<()> {
        self.span = node.span();

        match node.kind() {
            Expr => {
                if matches!(self.expr, ExprSupport::No) {
                    idx.error(Error::msg(
                        &node,
                        "expression is not supported during indexing",
                    ))?;
                    return Ok(());
                }

                let (mods, attrs, p) = node.parse(|p| {
                    let mods = p
                        .eat(Modifiers)
                        .parse(|p| Mods::parse(idx, p))?
                        .unwrap_or_default();
                    let attrs = attributes(idx, p)?;
                    Ok((mods, attrs, p.take_remaining()))
                })?;

                self.stack.try_push(State::Expr(mods, attrs))?;
                self.stack.try_push(State::Stream(p))?;
                return Ok(());
            }
            ExprMacroCall => {
                let builtin;

                if let [.., State::Expr(_, attrs), _] = &mut self.stack[..] {
                    builtin = attrs.builtin.take();
                } else {
                    builtin = None;
                }

                let (literal, is_builtin) = match builtin {
                    Some((_, literal)) => (literal, true),
                    _ => (BuiltInLiteral::No, false),
                };

                let call = |id| {
                    Ok::<_, alloc::Error>(ExpandMacroBuiltin {
                        id,
                        node: node.node_at(idx.source_id, idx.tree.clone()),
                        location: Location::new(idx.source_id, node.span()),
                        root: idx.root,
                        macro_depth: idx.macro_depth,
                        item: idx.item,
                        literal,
                    })
                };

                let id = if is_builtin {
                    idx.q
                        .insert_new_macro(move |id| Ok(DeferEntry::ExpandMacroBuiltin(call(id)?)))
                        .with_span(self.span)?
                } else {
                    for node in node.children() {
                        if node.kind() == Path {
                            node.replace(IndexedPath(idx.item.id));
                        }
                    }

                    idx.q
                        .insert_new_macro(move |id| Ok(DeferEntry::ExpandMacroCall(call(id)?)))
                        .with_span(self.span)?
                };

                node.replace(ExpandedMacro(id));
                return Ok(());
            }
            ExprClosure => {
                let async_token;
                let move_token;

                if let [.., State::Expr(mods, _), _] = &mut self.stack[..] {
                    async_token = mods.async_token.take();
                    move_token = mods.move_token.take();
                } else {
                    async_token = None;
                    move_token = None;
                }

                let guard = idx.push_id()?;
                idx.scopes.push()?;
                let item_meta = idx.insert_new_item(&self.span, Visibility::Inherited, &[])?;
                let item = idx.item.replace(item_meta.item);
                self.stack
                    .try_push(State::Closure(guard, item, async_token, move_token))?;
                node.replace(Closure(idx.item.id));
            }
            Path => {
                node.replace(IndexedPath(idx.item.id));
            }
            Block => {
                let async_token;
                let const_token;
                let move_token;

                if let [.., State::Expr(mods, _), _] = &mut self.stack[..] {
                    async_token = mods.async_token.take();
                    const_token = mods.const_token.take();
                    move_token = mods.move_token.take();
                } else {
                    async_token = None;
                    const_token = None;
                    move_token = None;
                }

                let guard = idx.push_id()?;
                let item_meta = idx.insert_new_item(&self.span, Visibility::Inherited, &[])?;
                let item = idx.item.replace(item_meta.item);

                let kind = match (async_token, const_token) {
                    (Some(const_token), Some(async_token)) => {
                        idx.error(Error::new(
                            const_token.span.join(async_token.span),
                            ErrorKind::FnConstAsyncConflict,
                        ))?;

                        ExprBlockKind::Default
                    }
                    (Some(..), None) => ExprBlockKind::Async,
                    (None, Some(..)) => ExprBlockKind::Const,
                    _ => ExprBlockKind::Default,
                };

                match kind {
                    ExprBlockKind::Default => {
                        self.stack.try_push(State::Block(guard, item))?;
                    }
                    ExprBlockKind::Const => {
                        self.stack
                            .try_push(State::ConstBlock(guard, item, node.clone()))?;
                        node.replace(ConstBlock(idx.item.id));
                    }
                    ExprBlockKind::Async => {
                        idx.scopes.push()?;
                        self.stack.try_push(State::AsyncBlock(
                            guard,
                            item,
                            node.span(),
                            move_token,
                        ))?;
                        node.replace(AsyncBlock(idx.item.id));
                    }
                }
            }
            ExprSelect | ExprAwait => {
                let l = idx.scopes.mark().with_span(self.span)?;
                l.awaits.try_push(node.span())?;
            }
            ExprYield => {
                let l = idx.scopes.mark().with_span(self.span)?;
                l.yields.try_push(node.span())?;
            }
            Item => {
                let result = node.parse(|p| {
                    let attrs = attributes(idx, p)?;
                    p.pump()?.parse(|p| self.item(idx, p, attrs))?;
                    Ok(())
                });

                if let Err(error) = result {
                    idx.error(error)?;
                }

                return Ok(());
            }
            _ => {}
        }

        let mut p = node.into_stream();

        if !p.is_eof() {
            self.stack.try_push(State::Stream(p)).with_span(self.span)?;
        }

        Ok(())
    }

    fn bare(&mut self, idx: &mut Indexer<'_, '_>, id: NodeId) -> Result<()> {
        let item_meta = idx
            .q
            .item_for("bare function", idx.item.id)
            .with_span(self.span)?;
        let layer = idx.scopes.pop().with_span(self.span)?;

        let call = match (layer.awaits.is_empty(), layer.yields.is_empty()) {
            (true, true) => Call::Immediate,
            (false, true) => Call::Async,
            (true, false) => Call::Generator,
            (false, false) => Call::Stream,
        };

        idx.q.index_and_build(indexing::Entry {
            item_meta,
            indexed: Indexed::Function(indexing::Function {
                ast: indexing::FunctionAst::Bare(idx.tree.node_at(idx.source_id, id)),
                call,
                is_instance: false,
                is_test: false,
                is_bench: false,
                impl_item: None,
                args: Vec::new(),
            }),
        })?;

        Ok(())
    }

    fn function(
        &mut self,
        idx: &mut Indexer<'_, '_>,
        index: usize,
        nested_item: Option<Span>,
    ) -> Result<(), Error> {
        let Some(f) = self.fns.pop() else {
            return Err(Error::msg(self.span, "missing function being indexed"));
        };

        if index != self.fns.len() {
            return Err(Error::msg(self.span, "function indexing mismatch"));
        }

        let item_meta = idx
            .q
            .item_for("function", idx.item.id)
            .with_span(self.span)?;

        let Function {
            node,
            name,
            attrs,
            mods,
        } = f;

        self.span = node.span();

        let (is_instance, args) = is_instance(&node).with_span(&node)?;
        let layer = idx.scopes.pop().with_span(self.span)?;

        if let (Some(const_token), Some(async_token)) = (mods.const_token, mods.async_token) {
            idx.error(Error::new(
                const_token.span.join(async_token.span),
                ErrorKind::FnConstAsyncConflict,
            ))?;
        };

        let call = validate_call(
            mods.const_token.is_some(),
            mods.async_token.is_some(),
            &layer,
        )?;

        let Some(call) = call else {
            idx.q.index_const_fn(
                item_meta,
                indexing::ConstFn::Node(node.node_at(idx.source_id, idx.tree.clone())),
            )?;
            return Ok(());
        };

        if let (Some(span), Some(_nested_span)) = (attrs.test, nested_item) {
            idx.error(Error::new(
                span,
                ErrorKind::NestedTest {
                    #[cfg(feature = "emit")]
                    nested_span: _nested_span,
                },
            ))?;
        }

        if let (Some(span), Some(_nested_span)) = (attrs.bench, nested_item) {
            idx.error(Error::new(
                span,
                ErrorKind::NestedBench {
                    #[cfg(feature = "emit")]
                    nested_span: _nested_span,
                },
            ))?;
        }

        let is_test = attrs.test.is_some();
        let is_bench = attrs.bench.is_some();

        if idx.item.impl_item.is_some() {
            if is_test {
                idx.error(Error::msg(
                    &node,
                    "the #[test] attribute is not supported on associated functions",
                ))?;
            }

            if is_bench {
                idx.error(Error::msg(
                    &node,
                    "the #[bench] attribute is not supported on associated functions",
                ))?;
            }
        }

        if is_instance && idx.item.impl_item.is_none() {
            idx.error(Error::new(&node, ErrorKind::InstanceFunctionOutsideImpl))?;
        };

        let entry = indexing::Entry {
            item_meta,
            indexed: Indexed::Function(indexing::Function {
                ast: indexing::FunctionAst::Node(
                    node.node_at(idx.source_id, idx.tree.clone()),
                    name,
                ),
                call,
                is_instance,
                is_test,
                is_bench,
                impl_item: idx.item.impl_item,
                args,
            }),
        };

        let is_exported = is_instance
            || item_meta.is_public(idx.q.pool) && nested_item.is_none()
            || is_test
            || is_bench;

        if is_exported {
            idx.q.index_and_build(entry)?;
        } else {
            idx.q.index(entry)?;
        }

        Ok(())
    }

    fn item(&mut self, idx: &mut Indexer<'_, '_>, p: &mut Stream<'a>, attrs: Attrs) -> Result<()> {
        let mods = p
            .eat(Modifiers)
            .parse(|p| Mods::parse(idx, p))?
            .unwrap_or_default();

        match p.kind() {
            ItemFn => {
                self.item_fn(idx, p, mods, attrs)?;
            }
            ItemMod => {
                self.item_mod(idx, p, mods, attrs)?;
            }
            ItemFileMod => {
                self.item_file_mod(idx, p, mods, attrs)?;
            }
            ItemImpl => {
                item_impl(idx, p, mods, attrs)?;
            }
            ItemStruct => {
                item_struct(idx, p, mods, attrs)?;
            }
            ItemEnum => {
                item_enum(idx, p, mods, attrs)?;
            }
            ItemConst => {
                self.item_const(idx, p, mods, attrs)?;
            }
            ItemUse => {
                item_use(idx, p, mods, attrs)?;
            }
            _ => {
                idx.error(p.expected("item"))?;
                p.ignore();
            }
        }

        Ok(())
    }

    fn item_fn(
        &mut self,
        idx: &mut Indexer<'_, '_>,
        p: &mut Stream<'a>,
        mods: Mods,
        attrs: Attrs,
    ) -> Result<(), Error> {
        let expr = replace(&mut self.expr, ExprSupport::Yes);

        p.expect(K![fn])?;

        let (guard, name) = push_name(idx, p, "function")?;
        let item_meta = idx.insert_new_item(&*p, mods.visibility, &attrs.docs)?;
        let item = idx.item.replace(item_meta.item);
        idx.scopes.push()?;

        let nested_item = idx.nested_item.replace(p.span());
        let index = self.fns.len();

        self.fns.try_push(Function {
            node: p.node(),
            name,
            attrs,
            mods,
        })?;

        self.stack
            .try_push(State::Function(guard, item, index, nested_item, expr))?;
        self.stack.try_push(State::Stream(p.take_remaining()))?;
        Ok(())
    }

    fn item_mod(
        &mut self,
        idx: &mut Indexer<'_, '_>,
        p: &mut Stream<'a>,
        mut mods: Mods,
        attrs: Attrs,
    ) -> Result<()> {
        p.expect(K![mod])?;
        let (guard, _) = push_name(idx, p, "module")?;

        let MaybeNode::Some(node) = p.eat(Block) else {
            idx.error(p.expected_peek(Block))?;
            return Ok(());
        };

        let (mod_item, mod_item_id) = idx.q.insert_mod(
            &idx.items,
            &DynLocation::new(idx.source_id, &*p),
            idx.item.module,
            mods.visibility.take(),
            &attrs.docs,
        )?;

        let item = idx.item.replace_module(mod_item, mod_item_id);

        self.stack.try_push(State::Module(guard, item))?;
        self.stack.try_push(State::Stream(node.into_stream()))?;

        mods.deny_all(idx)?;
        attrs.deny_non_docs(idx)?;
        Ok(())
    }

    fn item_file_mod(
        &mut self,
        idx: &mut Indexer<'_, '_>,
        p: &mut Stream<'_>,
        mut mods: Mods,
        attrs: Attrs,
    ) -> Result<()> {
        p.expect(K![mod])?;
        let (guard, _) = push_name(idx, p, "module")?;

        let (mod_item, mod_item_id) = idx.q.insert_mod(
            &idx.items,
            &DynLocation::new(idx.source_id, &*p),
            idx.item.module,
            mods.visibility.take(),
            &attrs.docs,
        )?;

        idx.items.pop(guard).with_span(&*p)?;

        let Some(root) = idx.root else {
            return Err(Error::new(&*p, ErrorKind::UnsupportedModuleSource));
        };

        let source =
            idx.q
                .source_loader
                .load(idx.q.sources, root, idx.q.pool.module_item(mod_item), &*p)?;

        if let Some(loaded) = idx.loaded.as_mut() {
            if let Some(_existing) = loaded.try_insert(mod_item, (idx.source_id, p.span()))? {
                return Err(Error::new(
                    &*p,
                    ErrorKind::ModAlreadyLoaded {
                        item: idx.q.pool.module_item(mod_item).try_to_owned()?,
                        #[cfg(feature = "emit")]
                        existing: _existing,
                    },
                ));
            }
        }

        let source_id = idx.q.sources.insert(source)?;

        idx.q
            .visitor
            .visit_mod(&DynLocation::new(source_id, &*p))
            .with_span(&*p)?;

        if let Some(queue) = idx.queue.as_mut() {
            queue.try_push_back(worker::Task::LoadFile {
                kind: worker::LoadFileKind::Module { root: idx.root },
                source_id,
                mod_item,
                mod_item_id,
            })?;
        }

        mods.deny_all(idx)?;
        attrs.deny_non_docs(idx)?;
        Ok(())
    }

    fn item_const(
        &mut self,
        idx: &mut Indexer<'_, '_>,
        p: &mut Stream<'a>,
        mut mods: Mods,
        attrs: Attrs,
    ) -> Result<()> {
        let expr = replace(&mut self.expr, ExprSupport::Yes);

        if mods.const_token.take().is_none() {
            idx.error(Error::msg(&*p, "missing `const` modifier"))?;
        }

        let (guard, _) = push_name(idx, p, "constant")?;

        p.expect(K![=])?;
        let value = p.expect(Expr)?;

        let item_meta = idx.insert_new_item(&value, mods.visibility.take(), &attrs.docs)?;

        let idx_item = idx.item.replace(item_meta.item);
        let last = idx.nested_item.replace(value.span());

        self.stack
            .try_push(State::Const(guard, idx_item, last, value.clone(), expr))?;

        self.stack.try_push(State::Stream(value.into_stream()))?;

        mods.deny_all(idx)?;
        attrs.deny_non_docs(idx)?;
        Ok(())
    }
}

/// Index the contents of a module known by its AST as a "file".
pub(crate) fn bare(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    let item_meta = idx.insert_new_item(p, Visibility::Public, &[])?;
    let item = idx.item.replace(item_meta.item);

    idx.scopes.push()?;

    inner_attributes(idx, p)?;

    let mut proc = Processor::new(p.span()).with_expr(ExprSupport::Yes);
    proc.stack.try_push(State::Bare(item, p.id()))?;
    proc.stack.try_push(State::Stream(p.take_remaining()))?;
    proc.process(idx)?;
    Ok(())
}

/// Index the contents of a module known by its AST as a "file".
pub(crate) fn file(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    inner_attributes(idx, p)?;

    let mut proc = Processor::new(p.span());
    proc.stack.try_push(State::Stream(p.take_remaining()))?;
    proc.process(idx)?;
    Ok(())
}

/// Index an item.
pub(crate) fn item(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>, attrs: Attrs) -> Result<()> {
    let mut proc = Processor::new(p.span());
    proc.item(idx, p, attrs)?;
    proc.process(idx)?;
    Ok(())
}

/// Index anything.
pub(crate) fn any(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    let mut proc = Processor::new(p.span()).with_expr(ExprSupport::Yes);
    proc.stack.try_push(State::Stream(p.take_remaining()))?;
    proc.process(idx)?;
    Ok(())
}

fn item_impl(
    idx: &mut Indexer<'_, '_>,
    p: &mut Stream<'_>,
    mods: Mods,
    attrs: Attrs,
) -> Result<()> {
    p.expect(K![impl])?;

    let MaybeNode::Some(node) = p.eat(Path) else {
        idx.error(p.expected_peek(Path))?;
        return Ok(());
    };

    node.replace(IndexedPath(idx.item.id));

    let mut functions = Vec::new();

    p.eat(Block).parse(|p| {
        p.eat(K!['{']);

        p.expect(BlockBody)?.parse(|p| {
            while let MaybeNode::Some(item) = p.eat(Item) {
                item.parse(|p| {
                    let attrs = attributes(idx, p)?;

                    p.pump()?.parse(|p| {
                        let mods = p
                            .eat(Modifiers)
                            .parse(|p| Mods::parse(idx, p))?
                            .unwrap_or_default();

                        mods.deny_all(idx)?;

                        match p.kind() {
                            ItemFn => {
                                functions.try_push((p.id(), attrs)).with_span(&*p)?;
                                p.ignore();
                            }
                            _ => {
                                idx.error(p.expected(ItemFn)).with_span(&*p)?;
                            }
                        }

                        Ok(())
                    })?;

                    Ok(())
                })?;
            }

            Ok(())
        })?;

        p.eat(K!['}']);
        Ok(())
    })?;

    let location = Location::new(idx.source_id, p.span());

    idx.q
        .inner
        .defer_queue
        .try_push_back(DeferEntry::ImplItem(ImplItem {
            kind: ImplItemKind::Node {
                path: node.node_at(idx.source_id, idx.tree.clone()),
                functions,
            },
            location,
            root: idx.root,
            nested_item: idx.nested_item,
            macro_depth: idx.macro_depth,
        }))?;

    mods.deny_all(idx)?;
    attrs.deny_non_docs(idx)?;
    Ok(())
}

fn item_struct(
    idx: &mut Indexer<'_, '_>,
    p: &mut Stream<'_>,
    mut mods: Mods,
    attrs: Attrs,
) -> Result<()> {
    p.expect(K![struct])?;
    let (guard, _) = push_name(idx, p, "struct")?;

    let item_meta = idx.insert_new_item(&*p, mods.visibility.take(), &attrs.docs)?;

    let fields = p.pump()?.parse(|p| fields(idx, p))?;

    idx.q.index_struct(item_meta, indexing::Struct { fields })?;

    idx.items.pop(guard).with_span(&*p)?;

    mods.deny_all(idx)?;
    attrs.deny_non_docs(idx)?;
    Ok(())
}

fn item_enum(
    idx: &mut Indexer<'_, '_>,
    p: &mut Stream<'_>,
    mut mods: Mods,
    attrs: Attrs,
) -> Result<()> {
    p.expect(K![enum])?;
    let (guard, _) = push_name(idx, p, "enum")?;

    let vis = mods.visibility.take();

    let enum_item_meta = idx.insert_new_item(&*p, vis, &attrs.docs)?;

    idx.q.index_enum(enum_item_meta)?;

    p.eat(K!['{']);

    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Variant) {
        comma.at_most_one(idx)?;

        let (item_meta, fields, attrs) = node.parse(|p| {
            let attrs = attributes(idx, p)?;

            let (guard, _) = push_name(idx, p, "variant")?;
            let item_meta = idx.insert_new_item(&*p, vis, &attrs.docs)?;
            let fields = p.pump()?.parse(|p| fields(idx, p))?;
            idx.items.pop(guard).with_span(&*p)?;

            Ok((item_meta, fields, attrs))
        })?;

        attrs.deny_non_docs(idx)?;

        let variant = indexing::Variant {
            enum_id: enum_item_meta.item,
            fields,
        };

        idx.q.index_variant(item_meta, variant)?;

        comma = p.remaining(idx, K![,])?;
    }

    comma.at_most_one(idx)?;
    p.eat(K!['}']);
    idx.items.pop(guard).with_span(&*p)?;

    mods.deny_all(idx)?;
    attrs.deny_non_docs(idx)?;
    Ok(())
}

fn item_use(
    idx: &mut Indexer<'_, '_>,
    p: &mut Stream<'_>,
    mut mods: Mods,
    attrs: Attrs,
) -> Result<()> {
    let import = Import {
        state: ImportState::Node(p.node().node_at(idx.source_id, idx.tree.clone())),
        kind: ImportKind::Global,
        visibility: mods.visibility.take(),
        module: idx.item.module,
        item: idx.items.item().try_to_owned()?,
        source_id: idx.source_id,
    };

    let span = p.span();

    import.process(&mut idx.q, &mut |task| {
        let Some(queue) = &mut idx.queue else {
            return Err(Error::msg(
                span,
                "deferred imports are not supported in this context",
            ));
        };

        queue.try_push_back(task)?;
        Ok(())
    })?;

    // Ignore remaining tokens since they will be processed by the
    // import.
    p.ignore();

    mods.deny_all(idx)?;
    attrs.deny_any(idx)?;
    Ok(())
}

fn push_name(
    idx: &mut Indexer<'_, '_>,
    p: &mut Stream<'_>,
    what: &'static str,
) -> Result<(Guard, Option<ast::Ident>)> {
    let (guard, ident) = if let Some(ident) = p.try_ast::<ast::Ident>()? {
        let name = ident.resolve(resolve_context!(idx.q))?;
        (idx.items.push_name(name.as_ref())?, Some(ident))
    } else {
        idx.error(Error::msg(
            p.peek_span().head(),
            try_format!("expected {what} name"),
        ))?;
        (idx.push_id()?, None)
    };

    Ok((guard, ident))
}

fn attributes(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    while let MaybeNode::Some(node) = p.eat(Attribute) {
        node.parse(|p| {
            let span = p.span();

            p.all([K![#], K!['[']])?;

            p.expect(TokenStream)?.parse(|p| {
                let ident = p.ast::<ast::Ident>()?;

                match ident.resolve(resolve_context!(idx.q))? {
                    "test" => {
                        if attrs.bench.is_some() {
                            idx.error(Error::msg(ident.span, "duplicate #[test] attribute"))?;
                        } else {
                            attrs.test = Some(ident.span);
                        }
                    }
                    "bench" => {
                        if attrs.bench.is_some() {
                            idx.error(Error::msg(ident.span, "duplicate #[bench] attribute"))?;
                        } else {
                            attrs.bench = Some(ident.span);
                        }
                    }
                    "doc" => {
                        p.expect(K![=])?;
                        let doc_string = p.ast::<ast::LitStr>()?;
                        attrs
                            .docs
                            .try_push(Doc { span, doc_string })
                            .with_span(&*p)?;
                    }
                    "builtin" => {
                        let mut literal = BuiltInLiteral::No;

                        if p.eat(K!['(']).is_some() {
                            while matches!(p.peek(), K![ident]) {
                                let ident = p.ast::<ast::Ident>()?;

                                match ident.resolve(resolve_context!(idx.q))? {
                                    "literal" => {
                                        literal = BuiltInLiteral::Yes(ident.span());
                                    }
                                    other => {
                                        idx.error(Error::msg(
                                            ident,
                                            try_format!("unsupported builtin option `{other}`"),
                                        ))?;
                                    }
                                }

                                p.remaining(idx, K![,])?.ignore(idx)?;
                            }

                            p.expect(K![')'])?;
                        }

                        if attrs.builtin.is_some() {
                            idx.error(Error::msg(ident.span, "duplicate #[builtin] attribute"))?;
                        } else {
                            attrs.builtin = Some((ident.span, literal));
                        }
                    }
                    name => {
                        idx.error(Error::msg(
                            ident,
                            try_format!("unsupported attribute `{name}`"),
                        ))?;
                    }
                }

                Ok(())
            })?;

            p.expect(K![']'])?;
            Ok(())
        })?;
    }

    Ok(attrs)
}

fn inner_attributes(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<()> {
    while let MaybeNode::Some(node) = p.eat(InnerAttribute) {
        node.parse(|p| {
            p.all([K![#], K![!], K!['[']])?;

            p.expect(TokenStream)?.parse(|p| {
                let ident = p.ast::<ast::Ident>()?;

                match ident.resolve(resolve_context!(idx.q))? {
                    "doc" => {
                        p.expect(K![=])?;

                        let str = p.ast::<ast::LitStr>()?;
                        let str = str.resolve(resolve_context!(idx.q))?;

                        let loc = DynLocation::new(idx.source_id, &*p);

                        let item = idx.q.pool.item(idx.item.id);
                        let hash = idx.q.pool.item_type_hash(idx.item.id);

                        idx.q
                            .visitor
                            .visit_doc_comment(&loc, item, hash, &str)
                            .with_span(&*p)?;
                    }
                    name => {
                        idx.error(Error::msg(
                            ident,
                            try_format!("unsupported attribute `{name}`"),
                        ))?;
                    }
                }

                Ok(())
            })?;

            p.expect(K![']'])?;
            Ok(())
        })?;
    }

    Ok(())
}

fn fields(idx: &mut Indexer<'_, '_>, p: &mut Stream<'_>) -> Result<meta::Fields> {
    match p.kind() {
        StructBody => {
            p.one(K!['{']).exactly_one(idx)?;
            let mut fields = Vec::new();
            let mut comma = Remaining::default();

            while let MaybeNode::Some(field) = p.eat(Field) {
                comma.exactly_one(idx)?;
                let name = field.parse(|p| p.ast::<ast::Ident>())?;
                let name = name.resolve(resolve_context!(idx.q))?;
                let position = fields.len();
                fields.try_push(meta::FieldMeta {
                    name: name.try_into()?,
                    position,
                })?;
                comma = p.remaining(idx, K![,])?;
            }

            comma.at_most_one(idx)?;
            p.one(K!['}']).exactly_one(idx)?;
            Ok(meta::Fields::Named(meta::FieldsNamed {
                fields: fields.try_into()?,
            }))
        }
        TupleBody => {
            p.one(K!['(']).exactly_one(idx)?;
            let mut count = 0;
            let mut comma = Remaining::default();

            while p.eat(Field).is_some() {
                comma.exactly_one(idx)?;
                count += 1;
                comma = p.remaining(idx, K![,])?;
            }

            comma.at_most_one(idx)?;
            p.one(K![')']).exactly_one(idx)?;
            Ok(meta::Fields::Unnamed(count))
        }
        EmptyBody => Ok(meta::Fields::Empty),
        _ => {
            idx.error(p.expected("struct body"))?;
            Ok(meta::Fields::Empty)
        }
    }
}

#[derive(Default, Debug)]
struct Mods {
    span: Span,
    visibility: Visibility,
    const_token: Option<ast::Const>,
    async_token: Option<ast::Async>,
    move_token: Option<ast::Move>,
}

impl Mods {
    /// Parse modifiers.
    fn parse(cx: &mut dyn Ignore<'_>, p: &mut Stream<'_>) -> Result<Mods> {
        let mut mods = Mods {
            span: p.span().head(),
            visibility: Visibility::Inherited,
            const_token: None,
            async_token: None,
            move_token: None,
        };

        loop {
            match p.peek() {
                K![pub] => {
                    mods.visibility = Visibility::Public;
                }
                ModifierSelf if mods.visibility == Visibility::Public => {
                    mods.visibility = Visibility::SelfValue;
                }
                ModifierSuper if mods.visibility == Visibility::Public => {
                    mods.visibility = Visibility::Super;
                }
                ModifierCrate if mods.visibility == Visibility::Public => {
                    mods.visibility = Visibility::Crate;
                }
                _ => {
                    break;
                }
            }

            mods.span = mods.span.join(p.pump()?.span());
        }

        while let Some(tok) = p.try_ast::<ast::Const>()? {
            if mods.const_token.is_some() {
                cx.error(Error::msg(tok, "duplicate `const` modifier"))?;
            } else {
                mods.const_token = Some(tok);
            }
        }

        while let Some(tok) = p.try_ast::<ast::Async>()? {
            if mods.async_token.is_some() {
                cx.error(Error::msg(tok, "duplicate `async` modifier"))?;
            } else {
                mods.async_token = Some(tok);
            }
        }

        while let Some(tok) = p.try_ast::<ast::Move>()? {
            if mods.move_token.is_some() {
                cx.error(Error::msg(tok, "duplicate `move` modifier"))?;
            } else {
                mods.move_token = Some(tok);
            }
        }

        Ok(mods)
    }

    /// Deny any existing modifiers.
    fn deny_all(self, cx: &mut dyn Ignore<'_>) -> Result<()> {
        if !matches!(self.visibility, Visibility::Inherited) {
            cx.error(Error::msg(self.span, "unsupported visibility modifier"))?;
        }

        if let Some(span) = self.const_token {
            cx.error(Error::msg(span, "unsupported `const` modifier"))?;
        }

        if let Some(span) = self.async_token {
            cx.error(Error::msg(span, "unsupported `async` modifier"))?;
        }

        Ok(())
    }
}
