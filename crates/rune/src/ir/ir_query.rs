use crate::ast;
use crate::parsing::Id;
use crate::query::{QueryConstFn, QueryError, Used};
use runestick::{CompileMeta, Item, Span};
use std::rc::Rc;

/// Query interface for the interpreter.
pub(crate) trait IrQuery {
    /// Query for the given meta.
    fn query_meta(
        &mut self,
        spanned: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError>;

    /// Get the template associated with the AST.
    fn template_for(&self, spanned: Span, id: Option<Id>) -> Result<Rc<ast::Template>, QueryError>;

    /// Query for the constant function related to the given id.
    fn const_fn_for(&self, spanned: Span, id: Option<Id>) -> Result<Rc<QueryConstFn>, QueryError>;
}
