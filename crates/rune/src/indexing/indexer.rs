use rust_alloc::rc::Rc;

use core::mem::replace;

use crate::alloc::{self, HashMap, VecDeque};
use crate::ast::{Span, Spanned};
use crate::compile::{
    self, Doc, DynLocation, Error, ErrorKind, ItemId, ItemMeta, ModId, Visibility,
};
use crate::grammar::{Ignore, Node, Tree};
use crate::query::Query;
use crate::runtime::Call;
use crate::worker::Task;
use crate::SourceId;

use super::{Guard, Items, Layer, Scopes};

pub(crate) struct Indexer<'a, 'arena> {
    /// Query engine.
    pub(crate) q: Query<'a, 'arena>,
    pub(crate) source_id: SourceId,
    pub(crate) items: Items,
    /// Helper to calculate details about an indexed scope.
    pub(crate) scopes: Scopes,
    /// The current item state.
    pub(crate) item: IndexItem,
    /// Indicates if indexer is nested privately inside of another item, and if
    /// so, the descriptive span of its declaration.
    ///
    /// Private items are nested declarations inside of for example fn
    /// declarations:
    ///
    /// ```text
    /// pub fn public() {
    ///     fn private() {
    ///     }
    /// }
    /// ```
    ///
    /// Then, `nested_item` would point to the span of `pub fn public`.
    pub(crate) nested_item: Option<Span>,
    /// Depth of expression macro expansion that we're currently in.
    pub(crate) macro_depth: usize,
    /// The root URL that the indexed file originated from.
    pub(crate) root: Option<SourceId>,
    /// Imports to process.
    pub(crate) queue: Option<&'a mut VecDeque<Task>>,
    /// Loaded modules.
    pub(crate) loaded: Option<&'a mut HashMap<ModId, (SourceId, Span)>>,
    /// The current tree being processed.
    pub(crate) tree: &'a Rc<Tree>,
}

impl<'a> Ignore<'a> for Indexer<'_, '_> {
    /// Report an error.
    fn error(&mut self, error: Error) -> alloc::Result<()> {
        self.q.diagnostics.error(self.source_id, error)
    }

    fn ignore(&mut self, _: Node<'a>) -> compile::Result<()> {
        Ok(())
    }
}

impl Indexer<'_, '_> {
    /// Push an identifier item.
    pub(super) fn push_id(&mut self) -> alloc::Result<Guard> {
        let id = self.q.pool.next_id(self.item.id);
        self.items.push_id(id)
    }

    /// Insert a new item at the current indexed location.
    pub(crate) fn insert_new_item(
        &mut self,
        span: &dyn Spanned,
        visibility: Visibility,
        docs: &[Doc],
    ) -> compile::Result<ItemMeta> {
        self.q.insert_new_item(
            &self.items,
            self.item.module,
            self.item.impl_item,
            &DynLocation::new(self.source_id, span),
            visibility,
            docs,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexItem {
    /// The current module being indexed.
    pub(crate) module: ModId,
    /// Whether the item has been inserted or not.
    pub(crate) id: ItemId,
    /// Set if we are inside of an impl self.
    pub(crate) impl_item: Option<ItemId>,
}

impl IndexItem {
    pub(crate) fn new(module: ModId, id: ItemId) -> Self {
        Self {
            module,
            id,
            impl_item: None,
        }
    }

    pub(crate) fn with_impl_item(module: ModId, id: ItemId, impl_item: ItemId) -> Self {
        Self {
            module,
            id,
            impl_item: Some(impl_item),
        }
    }

    /// Replace item we're currently in.
    #[tracing::instrument(skip(self), fields(self.module = ?self.module, self.id = ?self.id, self.impl_item = ?self.impl_item))]
    pub(super) fn replace(&mut self, id: ItemId) -> IndexItem {
        tracing::debug!("replacing item");

        IndexItem {
            module: self.module,
            id: replace(&mut self.id, id),
            impl_item: self.impl_item,
        }
    }

    /// Replace module id.
    pub(super) fn replace_module(&mut self, module: ModId, id: ItemId) -> IndexItem {
        IndexItem {
            module: replace(&mut self.module, module),
            id: replace(&mut self.id, id),
            impl_item: self.impl_item,
        }
    }
}

/// Construct the calling convention based on the parameters.
pub(super) fn validate_call(
    is_const: bool,
    is_async: bool,
    layer: &Layer,
) -> compile::Result<Option<Call>> {
    for span in &layer.awaits {
        if is_const {
            return Err(compile::Error::new(span, ErrorKind::AwaitInConst));
        }

        if !is_async {
            return Err(compile::Error::new(span, ErrorKind::AwaitOutsideAsync));
        }
    }

    for span in &layer.yields {
        if is_const {
            return Err(compile::Error::new(span, ErrorKind::YieldInConst));
        }
    }

    if is_const {
        return Ok(None);
    }

    Ok(match (!layer.yields.is_empty(), is_async) {
        (true, false) => Some(Call::Generator),
        (false, false) => Some(Call::Immediate),
        (true, true) => Some(Call::Stream),
        (false, true) => Some(Call::Async),
    })
}
