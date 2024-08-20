use core::mem::{replace, take};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, VecDeque};
use crate::ast;
use crate::ast::Spanned;
use crate::compile::{DynLocation, Error, ErrorKind, Location, ModId, Result, Visibility};
use crate::grammar::{Ignore, MaybeNode, NodeAt, Remaining, Stream, StreamBuf};
use crate::parse::Resolve;
use crate::query::{Query, QuerySource};
use crate::worker::{ImportKind, Task, WildcardImport};
use crate::{ItemBuf, SourceId};

use ast::Kind::*;

/// The state of an import.
#[derive(Debug)]
pub(crate) enum ImportState {
    Ast(Box<ast::ItemUse>),
    Node(NodeAt),
    Complete,
}

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) state: ImportState,
    pub(crate) kind: ImportKind,
    pub(crate) module: ModId,
    pub(crate) visibility: Visibility,
    pub(crate) item: ItemBuf,
    pub(crate) source_id: SourceId,
}

impl Import {
    /// Lookup a local identifier in the current context and query.
    fn lookup_local(&self, query: &Query<'_, '_>, local: &str) -> alloc::Result<ItemBuf> {
        let item = query.pool.module_item(self.module).extended(local)?;

        if let ImportKind::Local = self.kind {
            if query.contains_prefix(&item)? {
                return Ok(item);
            }
        }

        if query.context.contains_crate(local) {
            return ItemBuf::with_crate(local);
        }

        Ok(item)
    }

    /// Process the import, populating the unit.
    pub(crate) fn process(
        mut self,
        q: &mut Query<'_, '_>,
        add_task: &mut dyn FnMut(Task) -> Result<()>,
    ) -> Result<()> {
        let mut q = q.with_source_id(self.source_id);

        match replace(&mut self.state, ImportState::Complete) {
            ImportState::Ast(ast) => {
                self.process_ast(&mut q, add_task, Box::into_inner(ast))?;
            }
            ImportState::Node(node) => {
                if !node.parse(|p| self.process_node(&mut q, p, add_task))? {
                    self.kind = ImportKind::Local;
                    self.state = ImportState::Node(node);

                    if let Err(error) = add_task(Task::ExpandImport(self)) {
                        q.error(error)?;
                    }
                }
            }
            ImportState::Complete => {}
        }

        Ok(())
    }

    fn process_ast(
        mut self,
        q: &mut Query<'_, '_>,
        add_task: &mut dyn FnMut(Task) -> Result<()>,
        ast: ast::ItemUse,
    ) -> Result<()> {
        let (name, first, initial) = match self.kind {
            ImportKind::Global => {
                match ast.path.global {
                    Some(global) => match &ast.path.first {
                        ast::ItemUseSegment::PathSegment(ast::PathSegment::Ident(ident)) => {
                            let ident = ident.resolve(resolve_context!(q))?;
                            (ItemBuf::with_crate(ident)?, None, false)
                        }
                        _ => {
                            return Err(Error::new(global.span(), ErrorKind::UnsupportedGlobal));
                        }
                    },
                    // NB: defer non-local imports.
                    _ => {
                        self.kind = ImportKind::Local;
                        self.state = ImportState::Ast(Box::try_new(ast)?);
                        add_task(Task::ExpandImport(self))?;
                        return Ok(());
                    }
                }
            }
            ImportKind::Local => (ItemBuf::new(), Some(&ast.path.first), true),
        };

        let mut queue = VecDeque::new();

        queue.try_push_back((&ast.path, name, first, initial))?;

        while let Some((path, mut name, first, mut initial)) = queue.pop_front() {
            tracing::trace!("process one");

            let mut it = first
                .into_iter()
                .chain(path.segments.iter().map(|(_, s)| s));

            let complete = loop {
                let segment = match it.next() {
                    Some(segment) => segment,
                    None => break None,
                };

                // Only the first ever segment loaded counts as the initial
                // segment.
                let initial = take(&mut initial);

                match segment {
                    ast::ItemUseSegment::PathSegment(segment) => match segment {
                        ast::PathSegment::Ident(ident) => {
                            let ident = ident.resolve(resolve_context!(q))?;

                            if !initial {
                                name.push(ident)?;
                                continue;
                            }

                            name = self.lookup_local(q, ident)?;
                        }
                        ast::PathSegment::SelfType(self_type) => {
                            return Err(Error::new(
                                self_type.span(),
                                ErrorKind::ExpectedLeadingPathSegment,
                            ));
                        }
                        ast::PathSegment::SelfValue(self_value) => {
                            if !initial {
                                return Err(Error::new(
                                    self_value.span(),
                                    ErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = q.pool.module_item(self.module).try_to_owned()?;
                        }
                        ast::PathSegment::Crate(crate_token) => {
                            if !initial {
                                return Err(Error::new(
                                    crate_token,
                                    ErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = ItemBuf::new();
                        }
                        ast::PathSegment::Super(super_token) => {
                            if initial {
                                name = q.pool.module_item(self.module).try_to_owned()?;
                            }

                            if !name.pop() {
                                return Err(Error::new(super_token, ErrorKind::UnsupportedSuper));
                            }
                        }
                        ast::PathSegment::Generics(arguments) => {
                            return Err(Error::new(arguments, ErrorKind::UnsupportedGenerics));
                        }
                    },
                    ast::ItemUseSegment::Wildcard(star_token) => {
                        let mut wildcard_import = WildcardImport {
                            visibility: self.visibility,
                            from: self.item.try_clone()?,
                            name: name.try_clone()?,
                            location: Location::new(self.source_id, star_token.span()),
                            module: self.module,
                            found: false,
                        };

                        wildcard_import.process_global(q)?;
                        add_task(Task::ExpandWildcardImport(wildcard_import))?;
                        break Some(star_token.span());
                    }
                    ast::ItemUseSegment::Group(group) => {
                        for (path, _) in group {
                            if let Some(global) = &path.global {
                                return Err(Error::new(
                                    global.span(),
                                    ErrorKind::UnsupportedGlobal,
                                ));
                            }

                            queue.try_push_back((
                                path,
                                name.try_clone()?,
                                Some(&path.first),
                                initial,
                            ))?;
                        }

                        break Some(group.span());
                    }
                }
            };

            if let Some(segment) = it.next() {
                return Err(Error::new(segment, ErrorKind::IllegalUseSegment));
            }

            let alias = match &path.alias {
                Some((_, ident)) => {
                    if let Some(span) = complete {
                        return Err(Error::new(
                            span.join(ident.span()),
                            ErrorKind::UseAliasNotSupported,
                        ));
                    }

                    Some(*ident)
                }
                None => None,
            };

            if complete.is_none() {
                q.insert_import(
                    &DynLocation::new(self.source_id, path),
                    self.module,
                    self.visibility,
                    &self.item,
                    &name,
                    alias,
                    false,
                )?;
            }
        }

        Ok(())
    }

    fn process_node(
        &self,
        q: &mut QuerySource<'_, '_>,
        p: &mut Stream<'_>,
        add_task: &mut dyn FnMut(Task) -> Result<()>,
    ) -> Result<bool> {
        p.eat(Modifiers);
        p.expect(K![use])?;

        let global = p.eat(K![::]).span();

        let (item, first, has_component) = match (&self.kind, global) {
            (ImportKind::Global, Some(global)) => match p.peek() {
                K![ident] => {
                    let ident = p.ast::<ast::Ident>()?;
                    let ident = ident.resolve(resolve_context!(q))?;
                    (ItemBuf::with_crate(ident)?, false, true)
                }
                _ => {
                    return Err(Error::new(global.span(), ErrorKind::UnsupportedGlobal));
                }
            },
            (ImportKind::Global, None) => {
                // Ignore remaining, since we will be processed in the global
                // queue.
                p.ignore();
                return Ok(false);
            }
            _ => (ItemBuf::new(), true, false),
        };

        let mut queue = VecDeque::new();

        queue.try_push_back((p.take_remaining(), item, first, has_component))?;

        while let Some((p, item, first, has_component)) = queue.pop_front() {
            tracing::trace!("process one");

            let result = p.parse(|p| {
                self.handle_import(
                    q,
                    p,
                    item,
                    first,
                    has_component,
                    add_task,
                    &mut |p, name| queue.try_push_back((p, name, false, false)),
                )
            });

            if let Err(error) = result {
                q.diagnostics.error(self.source_id, error)?;
            }
        }

        Ok(true)
    }

    fn handle_import<'a>(
        &self,
        q: &mut QuerySource<'_, '_>,
        p: &mut Stream<'a>,
        mut item: ItemBuf,
        mut first: bool,
        mut has_component: bool,
        add_task: &mut dyn FnMut(Task) -> Result<()>,
        enqueue: &mut dyn FnMut(StreamBuf<'a>, ItemBuf) -> alloc::Result<()>,
    ) -> Result<()> {
        let complete = loop {
            if p.is_eof() {
                break None;
            }

            // Only the first ever segment loaded counts as the initial
            // segment.
            let initial = take(&mut first);

            if has_component {
                if p.peek() == K![as] {
                    break None;
                }

                p.expect(K![::])?;
            }

            match p.peek() {
                K![ident] => {
                    let ident = p.ast::<ast::Ident>()?;
                    let ident = ident.resolve(resolve_context!(q))?;

                    if !initial {
                        item.push(ident)?;
                    } else {
                        item = self.lookup_local(q, ident)?;
                    }
                }
                K![Self] => {
                    let node = p.pump()?;

                    return Err(Error::new(node, ErrorKind::ExpectedLeadingPathSegment));
                }
                K![self] => {
                    let node = p.pump()?;

                    if !initial {
                        return Err(Error::new(node, ErrorKind::ExpectedLeadingPathSegment));
                    }

                    item = q.pool.module_item(self.module).try_to_owned()?;
                }
                K![crate] => {
                    let node = p.pump()?;

                    if !initial {
                        return Err(Error::new(node, ErrorKind::ExpectedLeadingPathSegment));
                    }

                    item = ItemBuf::new();
                }
                K![super] => {
                    let node = p.pump()?;

                    if initial {
                        item = q.pool.module_item(self.module).try_to_owned()?;
                    }

                    if !item.pop() {
                        return Err(Error::new(node, ErrorKind::UnsupportedSuper));
                    }
                }
                PathGenerics => {
                    let node = p.pump()?;
                    return Err(Error::new(node, ErrorKind::UnsupportedGenerics));
                }
                K![*] => {
                    let node = p.pump()?;

                    let mut wildcard_import = WildcardImport {
                        visibility: self.visibility,
                        from: self.item.try_clone()?,
                        name: item.try_clone()?,
                        location: Location::new(self.source_id, node.span()),
                        module: self.module,
                        found: false,
                    };

                    wildcard_import.process_global(q)?;

                    if let Err(error) = add_task(Task::ExpandWildcardImport(wildcard_import)) {
                        q.diagnostics.error(self.source_id, error)?;
                    }

                    break Some(node.span());
                }
                ItemUseGroup => {
                    let node = p.pump()?;
                    let group = Some(node.span());

                    node.parse(|p| {
                        p.expect(K!['{'])?;

                        let mut comma = Remaining::default();

                        while let MaybeNode::Some(node) = p.eat(ItemUsePath) {
                            comma.exactly_one(q)?;

                            node.parse(|p| {
                                if let Some(global) = p.eat(K![::]).span() {
                                    return Err(Error::new(global, ErrorKind::UnsupportedGlobal));
                                }

                                enqueue(p.take_remaining(), item.try_clone()?)?;
                                Ok(())
                            })?;

                            comma = p.one(K![,]);
                        }

                        comma.at_most_one(q)?;
                        p.expect(K!['}'])?;
                        Ok(())
                    })?;

                    break group;
                }
                _ => {
                    return Err(Error::new(p.peek_span(), ErrorKind::IllegalUseSegment));
                }
            }

            has_component = true;
        };

        let alias = if let MaybeNode::Some(node) = p.eat(K![as]) {
            if complete.is_some() {
                return Err(Error::new(node, ErrorKind::UseAliasNotSupported));
            }

            Some(p.ast::<ast::Ident>()?)
        } else {
            None
        };

        if let Some(span) = p.remaining_span() {
            return Err(Error::new(span, ErrorKind::IllegalUseSegment));
        }

        if complete.is_none() {
            q.insert_import(
                &DynLocation::new(self.source_id, p),
                self.module,
                self.visibility,
                &self.item,
                &item,
                alias,
                false,
            )?;
        }

        Ok(())
    }
}
