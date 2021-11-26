use crate::ast;
use crate::ast::Spanned;
use crate::compile::{CompileError, CompileErrorKind, CompileResult, Item, ModMeta, Visibility};
use crate::parse::Resolve;
use crate::query::Query;
use crate::worker::{ImportKind, Task, WildcardImport};
use crate::{Context, SourceId};
use std::collections::VecDeque;
use std::sync::Arc;

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) kind: ImportKind,
    pub(crate) module: Arc<ModMeta>,
    pub(crate) visibility: Visibility,
    pub(crate) item: Item,
    pub(crate) source_id: SourceId,
    pub(crate) ast: Box<ast::ItemUse>,
}

impl Import {
    /// Lookup a local identifier in the current context and query.
    fn lookup_local(&self, context: &Context, query: &Query, local: &str) -> Item {
        let item = self.module.item.extended(local);

        if let ImportKind::Local = self.kind {
            if query.contains_prefix(&item) {
                return item;
            }
        }

        if context.contains_crate(local) {
            return Item::with_crate(local);
        }

        item
    }

    /// Process the import, populating the unit.
    pub(crate) fn process(
        mut self,
        context: &Context,
        q: &mut Query,
        add_task: &mut impl FnMut(Task),
    ) -> CompileResult<()> {
        let (name, first, initial) = match self.kind {
            ImportKind::Global => {
                match self.ast.path.global {
                    Some(global) => match &self.ast.path.first {
                        ast::ItemUseSegment::PathSegment(ast::PathSegment::Ident(ident)) => {
                            let ident = ident.resolve(q.storage(), q.sources)?;
                            (Item::with_crate(ident), None, false)
                        }
                        _ => {
                            return Err(CompileError::new(
                                global.span(),
                                CompileErrorKind::UnsupportedGlobal,
                            ));
                        }
                    },
                    // NB: defer non-local imports.
                    _ => {
                        self.kind = ImportKind::Local;
                        add_task(Task::ExpandImport(self));
                        return Ok(());
                    }
                }
            }
            ImportKind::Local => (Item::new(), Some(&self.ast.path.first), true),
        };

        let mut queue = VecDeque::new();

        queue.push_back((&self.ast.path, name, first, initial));

        while let Some((path, mut name, first, mut initial)) = queue.pop_front() {
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
                let initial = std::mem::take(&mut initial);

                match segment {
                    ast::ItemUseSegment::PathSegment(segment) => match segment {
                        ast::PathSegment::Ident(ident) => {
                            let ident = ident.resolve(q.storage(), q.sources)?;

                            if !initial {
                                name.push(ident);
                                continue;
                            }

                            name = self.lookup_local(context, q, &*ident);
                        }
                        ast::PathSegment::SelfType(self_type) => {
                            return Err(CompileError::new(
                                self_type.span(),
                                CompileErrorKind::ExpectedLeadingPathSegment,
                            ));
                        }
                        ast::PathSegment::SelfValue(self_value) => {
                            if !initial {
                                return Err(CompileError::new(
                                    self_value.span(),
                                    CompileErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = self.module.item.clone();
                        }
                        ast::PathSegment::Crate(crate_token) => {
                            if !initial {
                                return Err(CompileError::new(
                                    crate_token,
                                    CompileErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = Item::new();
                        }
                        ast::PathSegment::Super(super_token) => {
                            if initial {
                                name = self.module.item.clone();
                            }

                            name.pop().ok_or_else(|| {
                                CompileError::new(super_token, CompileErrorKind::UnsupportedSuper)
                            })?;
                        }
                        ast::PathSegment::Generics(arguments) => {
                            return Err(CompileError::new(
                                arguments,
                                CompileErrorKind::UnsupportedGenerics,
                            ));
                        }
                    },
                    ast::ItemUseSegment::Wildcard(star_token) => {
                        let mut wildcard_import = WildcardImport {
                            visibility: self.visibility,
                            from: self.item.clone(),
                            name: name.clone(),
                            span: star_token.span(),
                            source_id: self.source_id,
                            module: self.module.clone(),
                            found: false,
                        };

                        wildcard_import.process_global(q, context)?;
                        add_task(Task::ExpandWildcardImport(wildcard_import));
                        break Some(star_token.span());
                    }
                    ast::ItemUseSegment::Group(group) => {
                        for (path, _) in group {
                            if let Some(global) = &path.global {
                                return Err(CompileError::new(
                                    global.span(),
                                    CompileErrorKind::UnsupportedGlobal,
                                ));
                            }

                            queue.push_back((path, name.clone(), Some(&path.first), initial));
                        }

                        break Some(group.span());
                    }
                }
            };

            if let Some(segment) = it.next() {
                return Err(CompileError::new(
                    segment,
                    CompileErrorKind::IllegalUseSegment,
                ));
            }

            let alias = match &path.alias {
                Some((_, ident)) => {
                    if let Some(span) = complete {
                        return Err(CompileError::new(
                            span.join(ident.span()),
                            CompileErrorKind::UseAliasNotSupported,
                        ));
                    }

                    Some(*ident)
                }
                None => None,
            };

            if complete.is_none() {
                q.insert_import(
                    self.source_id,
                    path.span(),
                    &self.module,
                    self.visibility,
                    self.item.clone(),
                    name,
                    alias,
                    false,
                )?;
            }
        }

        Ok(())
    }
}
