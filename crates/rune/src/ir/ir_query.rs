use crate::parsing::Id;
use crate::query::{BuiltInMacro, QueryConstFn, QueryError, Used};
use runestick::{CompileMeta, Item, Span};
use std::sync::Arc;

/// Query interface for the interpreter.
pub(crate) trait IrQuery {
    /// Query for the given meta.
    fn query_meta(
        &mut self,
        spanned: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError>;

    /// Get resolved internal macro with the given id.
    fn builtin_macro_for(
        &self,
        spanned: Span,
        id: Option<Id>,
    ) -> Result<Arc<BuiltInMacro>, QueryError>;

    /// Query for the constant function related to the given id.
    fn const_fn_for(&self, spanned: Span, id: Option<Id>) -> Result<Arc<QueryConstFn>, QueryError>;
}
