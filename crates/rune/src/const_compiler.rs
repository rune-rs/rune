use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::eval::{Eval as _, Used};
use crate::query::Query;
use crate::Resolve;
use crate::{CompileError, CompileErrorKind};
use runestick::{CompileMetaKind, ConstValue, Item, Source, Span};

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    /// Const expression that have been resolved.
    pub(crate) resolved: HashMap<Item, ConstValue>,
    /// Constant expressions being processed.
    pub(crate) processing: HashSet<Item>,
}

/// The compiler phase which evaluates constants.
pub(crate) struct ConstCompiler<'a> {
    /// The item where the constant expression is located.
    pub(crate) item: Item,
    /// Source file used in processing.
    pub(crate) source: &'a Source,
    /// Query engine to look for constant expressions.
    pub(crate) query: &'a mut Query,
}

impl<'a> ConstCompiler<'a> {
    /// Resolve the given resolvable value.
    pub(crate) fn resolve<T>(&self, value: &T) -> Result<T::Output, CompileError>
    where
        T: Resolve<'a>,
    {
        Ok(value.resolve(&self.query.storage, self.source)?)
    }

    /// Outer evaluation for an expression which performs caching into `consts`.
    pub(crate) fn eval_expr(
        &mut self,
        expr: &ast::Expr,
        used: Used,
    ) -> Result<ConstValue, CompileError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.query.consts.borrow().resolved.get(&self.item).cloned() {
            return Ok(const_value);
        }

        if !self
            .query
            .consts
            .borrow_mut()
            .processing
            .insert(self.item.clone())
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
        }

        let const_value = match self.eval(expr, used)? {
            Some(const_value) => const_value,
            None => {
                return Err(CompileError::new(expr, CompileErrorKind::NotConst));
            }
        };

        if self
            .query
            .consts
            .borrow_mut()
            .resolved
            .insert(self.item.clone(), const_value.clone())
            .is_some()
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
        }

        Ok(const_value)
    }

    /// Resolve the given constant value from the block scope.
    ///
    /// This looks up `const <ident> = <expr>` and evaluates them while caching
    /// their result.
    pub(crate) fn resolve_var(
        &mut self,
        ident: &str,
        span: Span,
        used: Used,
    ) -> Result<ConstValue, CompileError> {
        let mut base = self.item.clone();

        while !base.is_empty() {
            base.pop();
            let item = base.extended(ident);

            if let Some(const_value) = self.query.consts.borrow().resolved.get(&item).cloned() {
                return Ok(const_value);
            }

            let meta = match self.query.query_meta_with_use(&item, used)? {
                Some(meta) => meta,
                None => continue,
            };

            match &meta.kind {
                CompileMetaKind::Const { const_value, .. } => return Ok(const_value.clone()),
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedMetaConst { meta },
                    ));
                }
            }
        }

        Err(CompileError::new(span, CompileErrorKind::NotConst))
    }
}
