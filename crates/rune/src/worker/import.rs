use core::mem::take;

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, VecDeque};
use crate::ast;
use crate::ast::Spanned;
use crate::compile::{self, DynLocation, ErrorKind, ItemBuf, Location, ModId, Visibility};
use crate::parse::Resolve;
use crate::query::Query;
use crate::worker::{ImportKind, Task, WildcardImport};
use crate::SourceId;

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) kind: ImportKind,
    pub(crate) module: ModId,
    pub(crate) visibility: Visibility,
    pub(crate) item: ItemBuf,
    pub(crate) source_id: SourceId,
    pub(crate) ast: Box<ast::ItemUse>,
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
        add_task: &mut impl FnMut(Task) -> compile::Result<()>,
    ) -> compile::Result<()> {
        let (name, first, initial) = match self.kind {
            ImportKind::Global => {
                match self.ast.path.global {
                    Some(global) => match &self.ast.path.first {
                        ast::ItemUseSegment::PathSegment(ast::PathSegment::Ident(ident)) => {
                            let ident = ident.resolve(resolve_context!(q))?;
                            (ItemBuf::with_crate(ident)?, None, false)
                        }
                        _ => {
                            return Err(compile::Error::new(
                                global.span(),
                                ErrorKind::UnsupportedGlobal,
                            ));
                        }
                    },
                    // NB: defer non-local imports.
                    _ => {
                        self.kind = ImportKind::Local;
                        add_task(Task::ExpandImport(self))?;
                        return Ok(());
                    }
                }
            }
            ImportKind::Local => (ItemBuf::new(), Some(&self.ast.path.first), true),
        };

        let mut queue = VecDeque::new();

        queue.try_push_back((&self.ast.path, name, first, initial))?;

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
                            return Err(compile::Error::new(
                                self_type.span(),
                                ErrorKind::ExpectedLeadingPathSegment,
                            ));
                        }
                        ast::PathSegment::SelfValue(self_value) => {
                            if !initial {
                                return Err(compile::Error::new(
                                    self_value.span(),
                                    ErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = q.pool.module_item(self.module).try_to_owned()?;
                        }
                        ast::PathSegment::Crate(crate_token) => {
                            if !initial {
                                return Err(compile::Error::new(
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

                            name.pop()?.ok_or_else(|| {
                                compile::Error::new(super_token, ErrorKind::UnsupportedSuper)
                            })?;
                        }
                        ast::PathSegment::Generics(arguments) => {
                            return Err(compile::Error::new(
                                arguments,
                                ErrorKind::UnsupportedGenerics,
                            ));
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
                                return Err(compile::Error::new(
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
                return Err(compile::Error::new(segment, ErrorKind::IllegalUseSegment));
            }

            let alias = match &path.alias {
                Some((_, ident)) => {
                    if let Some(span) = complete {
                        return Err(compile::Error::new(
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
                    self.item.try_clone()?,
                    name,
                    alias,
                    false,
                )?;
            }
        }

        Ok(())
    }
}
