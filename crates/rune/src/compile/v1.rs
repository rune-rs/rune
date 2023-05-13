use crate::no_std::prelude::*;

use crate::ast::Span;
use crate::compile::context::ContextMeta;
use crate::compile::ir;
use crate::compile::meta;
use crate::compile::{
    self, Assembly, CompileErrorKind, IrBudget, IrCompiler, IrInterpreter, ItemId, ItemMeta,
    Location, Options, QueryErrorKind, WithSpan,
};
use crate::hir;
use crate::query::{ConstFn, Named, Query, Used};
use crate::runtime::{ConstValue, Inst};
use crate::{Context, Diagnostics, Hash, SourceId};

pub(crate) mod assemble;
mod loops;
mod scopes;

pub(crate) use self::loops::{Loop, Loops};
pub(crate) use self::scopes::{Scope, ScopeGuard, Scopes, Var};

/// Generic parameters.
#[derive(Default)]
pub(crate) struct GenericsParameters {
    trailing: usize,
    parameters: [Option<Hash>; 2],
}

impl GenericsParameters {
    fn is_empty(&self) -> bool {
        self.parameters.iter().all(|p| p.is_none())
    }

    fn as_boxed(&self) -> Box<[Option<Hash>]> {
        self.parameters.iter().copied().collect()
    }
}

impl AsRef<GenericsParameters> for GenericsParameters {
    #[inline]
    fn as_ref(&self) -> &GenericsParameters {
        self
    }
}

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Needs {
    Type,
    Value,
    None,
}

impl Needs {
    /// Test if any sort of value is needed.
    pub(crate) fn value(self) -> bool {
        matches!(self, Self::Type | Self::Value)
    }
}

pub(crate) struct Assembler<'a> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// The context we are compiling for.
    pub(crate) context: &'a Context,
    /// Query system to compile required items.
    pub(crate) q: Query<'a>,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation warnings.
    pub(crate) diagnostics: &'a mut Diagnostics,
}

enum ContextMatch<'this, 'm> {
    Context(&'m ContextMeta, Hash),
    Meta(&'this meta::Meta),
    None,
}

impl<'a> Assembler<'a> {
    // Pick private metadata to compile for the item.
    fn select_context_meta<'this, 'm>(
        &'this self,
        item: ItemId,
        metas: impl Iterator<Item = &'m ContextMeta> + Clone,
        parameters: &GenericsParameters,
    ) -> Result<ContextMatch<'this, 'm>, Box<QueryErrorKind>> {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Kind {
            None,
            Type,
            Function,
            AssociatedFunction,
        }

        /// Determine how the collection of generic parameters applies to the
        /// returned context meta.
        fn determine_kind<'m>(metas: impl Iterator<Item = &'m ContextMeta>) -> Option<Kind> {
            let mut kind = Kind::None;

            for meta in metas {
                let alt = match &meta.kind {
                    meta::Kind::Enum { .. }
                    | meta::Kind::Struct { .. }
                    | meta::Kind::Type { .. } => Kind::Type,
                    meta::Kind::Function { .. } => Kind::Function,
                    meta::Kind::AssociatedFunction { .. } => Kind::AssociatedFunction,
                    _ => {
                        continue;
                    }
                };

                if matches!(kind, Kind::None) {
                    kind = alt;
                    continue;
                }

                if kind != alt {
                    return None;
                }
            }

            Some(kind)
        }

        fn build_parameters(kind: Kind, p: &GenericsParameters) -> Option<Hash> {
            let hash = match (kind, p.trailing, p.parameters) {
                (_, 0, _) => Hash::EMPTY,
                (Kind::Type, 1, [Some(ty), None]) => Hash::EMPTY.with_type_parameters(ty),
                (Kind::Function, 1, [Some(f), None]) => Hash::EMPTY.with_function_parameters(f),
                (Kind::AssociatedFunction, 1, [Some(f), None]) => {
                    Hash::EMPTY.with_function_parameters(f)
                }
                (Kind::AssociatedFunction, 2, [Some(ty), f]) => Hash::EMPTY
                    .with_type_parameters(ty)
                    .with_function_parameters(f.unwrap_or(Hash::EMPTY)),
                _ => {
                    return None;
                }
            };

            Some(hash)
        }

        if let Some(parameters) =
            determine_kind(metas.clone()).and_then(|kind| build_parameters(kind, parameters))
        {
            if let Some(meta) = self.q.get_meta(item, parameters) {
                return Ok(ContextMatch::Meta(meta));
            }

            // If there is a single item matching the specified generic hash, pick
            // it.
            let mut it = metas
                .clone()
                .filter(|i| !matches!(i.kind, meta::Kind::Macro | meta::Kind::Module))
                .filter(|i| i.kind.as_parameters() == parameters);

            if let Some(meta) = it.next() {
                if it.next().is_none() {
                    return Ok(ContextMatch::Context(meta, parameters));
                }
            } else {
                return Ok(ContextMatch::None);
            }
        }

        if metas.clone().next().is_none() {
            return Ok(ContextMatch::None);
        }

        Err(Box::new(QueryErrorKind::AmbiguousContextItem {
            item: self.q.pool.item(item).to_owned(),
            infos: metas.map(|i| i.info()).collect(),
        }))
    }

    /// Access the meta for the given language item.
    pub fn try_lookup_meta(
        &mut self,
        span: Span,
        item: ItemId,
        parameters: &GenericsParameters,
    ) -> compile::Result<Option<meta::Meta>> {
        tracing::trace!("lookup meta: {:?}", item);

        if parameters.is_empty() {
            if let Some(meta) = self.q.query_meta(span, item, Default::default())? {
                tracing::trace!("found in query: {:?}", meta);
                self.q.visitor.visit_meta(
                    Location::new(self.source_id, span),
                    meta.as_meta_ref(self.q.pool),
                );
                return Ok(Some(meta));
            }
        }

        let Some(metas) = self.context.lookup_meta(self.q.pool.item(item)) else {
            return Ok(None);
        };

        let (meta, parameters) = match self
            .select_context_meta(item, metas, parameters)
            .with_span(span)?
        {
            ContextMatch::None => return Ok(None),
            ContextMatch::Meta(meta) => return Ok(Some(meta.clone())),
            ContextMatch::Context(meta, parameters) => (meta, parameters),
        };

        let Some(item) = &meta.item else {
            return Err(compile::Error::new(span,
            QueryErrorKind::MissingItem {
                hash: meta.hash,
            }));
        };

        let meta = meta::Meta {
            context: true,
            hash: meta.hash,
            item_meta: ItemMeta {
                id: Default::default(),
                location: Default::default(),
                item: self.q.pool.alloc_item(item),
                visibility: Default::default(),
                module: Default::default(),
            },
            kind: meta.kind.clone(),
            source: None,
            parameters,
        };

        self.q.insert_meta(meta.clone()).with_span(span)?;

        tracing::trace!("Found in context: {:?}", meta);

        self.q.visitor.visit_meta(
            Location::new(self.source_id, span),
            meta.as_meta_ref(self.q.pool),
        );

        Ok(Some(meta))
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(
        &mut self,
        span: Span,
        item: ItemId,
        parameters: impl AsRef<GenericsParameters>,
    ) -> compile::Result<meta::Meta> {
        let parameters = parameters.as_ref();

        if let Some(meta) = self.try_lookup_meta(span, item, parameters)? {
            return Ok(meta);
        }

        let kind = if !parameters.parameters.is_empty() {
            CompileErrorKind::MissingItemParameters {
                item: self.q.pool.item(item).to_owned(),
                parameters: parameters.as_boxed(),
            }
        } else {
            CompileErrorKind::MissingItem {
                item: self.q.pool.item(item).to_owned(),
            }
        };

        Err(compile::Error::new(span, kind))
    }

    /// Pop locals by simply popping them.
    pub(crate) fn locals_pop(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(Inst::Pop, span);
            }
            count => {
                self.asm.push(Inst::PopN { count }, span);
            }
        }
    }

    /// Clean up local variables by preserving the value that is on top and
    /// popping the rest.
    ///
    /// The clean operation will preserve the value that is on top of the stack,
    /// and pop the values under it.
    pub(crate) fn locals_clean(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(Inst::Clean { count }, span);
            }
        }
    }

    /// Convert an [ast::Path] into a [Named] item.
    pub(crate) fn convert_path<'hir>(
        &mut self,
        path: &'hir hir::Path<'hir>,
    ) -> compile::Result<Named<'hir>> {
        self.q.convert_path(self.context, path)
    }

    /// Clean the last scope.
    pub(crate) fn clean_last_scope(
        &mut self,
        span: Span,
        expected: ScopeGuard,
        needs: Needs,
    ) -> compile::Result<()> {
        let scope = self.scopes.pop(expected, span)?;

        if needs.value() {
            self.locals_clean(scope.local_var_count, span);
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    pub(crate) fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }

    /// Calling a constant function by id and return the resuling value.
    pub(crate) fn call_const_fn(
        &mut self,
        span: Span,
        meta: &meta::Meta,
        from: &ItemMeta,
        query_const_fn: &ConstFn,
        args: &[hir::Expr<'_>],
    ) -> compile::Result<ConstValue> {
        if query_const_fn.ir_fn.args.len() != args.len() {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::UnsupportedArgumentCount {
                    meta: meta.info(self.q.pool),
                    expected: query_const_fn.ir_fn.args.len(),
                    actual: args.len(),
                },
            ));
        }

        let mut compiler = IrCompiler {
            source_id: self.source_id,
            q: self.q.borrow(),
        };

        let mut compiled = Vec::new();

        // TODO: precompile these and fetch using opaque id?
        for (hir, name) in args.iter().zip(&query_const_fn.ir_fn.args) {
            compiled.push((ir::compiler::expr(hir, &mut compiler)?, name));
        }

        let mut interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            module: from.module,
            item: from.item,
            q: self.q.borrow(),
        };

        for (ir, name) in compiled {
            let value = interpreter.eval_value(&ir, Used::Used)?;
            interpreter.scopes.decl(name, value).with_span(span)?;
        }

        interpreter.module = query_const_fn.item_meta.module;
        interpreter.item = query_const_fn.item_meta.item;
        let value = interpreter.eval_value(&query_const_fn.ir_fn.ir, Used::Used)?;
        value.into_const(span)
    }
}
