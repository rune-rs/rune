use crate::collections::HashMap;
use crate::indexing::Visibility;
use crate::query::{ImportEntryStep, QueryError, QueryErrorKind, QueryItem, QueryMod};
use crate::shared::Location;
use crate::Id;
use runestick::{Item, Names, Span};
use std::rc::Rc;

/// Broken out substruct of `Query` to handle imports.
#[derive(Clone, Default)]
pub(crate) struct Imports {
    /// Prelude from the prelude.
    pub(super) prelude: HashMap<Box<str>, Item>,
    /// All available names in the context.
    pub(super) names: Names<(NameKind, Location)>,
    /// Associated between `id` and `Item`. Use to look up items through
    /// `item_for` with an opaque id.
    ///
    /// These items are associated with AST elements, and encodoes the item path
    /// that the AST element was indexed.
    pub(super) items: HashMap<Id, Rc<QueryItem>>,
    /// Modules and associated metadata.
    pub(super) modules: HashMap<Item, Rc<QueryMod>>,
    /// Reverse lookup for items to reduce the number of items used.
    pub(super) items_rev: HashMap<Item, Rc<QueryItem>>,
}

impl Imports {
    /// Check that the given item is accessible from the given module.
    pub(crate) fn check_access_to(
        &self,
        spanned: Span,
        from: &QueryMod,
        chain: &mut Vec<ImportEntryStep>,
        module: &QueryMod,
        location: Location,
        visibility: Visibility,
        item: &Item,
    ) -> Result<(), QueryError> {
        let (common, tree) = from.item.ancestry(&module.item);
        let mut current_module = common.clone();

        // Check each module from the common ancestrly to the module.
        for c in &tree {
            current_module.push(c);

            let m = self.modules.get(&current_module).ok_or_else(|| {
                QueryError::new(
                    spanned,
                    QueryErrorKind::MissingMod {
                        item: current_module.clone(),
                    },
                )
            })?;

            if !m.visibility.is_visible(&common, &current_module) {
                return Err(QueryError::new(
                    spanned,
                    QueryErrorKind::NotVisibleMod {
                        chain: into_chain(std::mem::take(chain)),
                        location: m.location,
                        visibility: m.visibility,
                        item: current_module,
                        from: from.item.clone(),
                    },
                ));
            }
        }

        if !visibility.is_visible_inside(&common, &module.item) {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::NotVisible {
                    chain: into_chain(std::mem::take(chain)),
                    location,
                    visibility,
                    item: item.clone(),
                    from: from.item.clone(),
                },
            ));
        }

        Ok(())
    }
}

/// An imported entry.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// The location of the import.
    pub location: Location,
    /// The visibility of the import.
    pub visibility: Visibility,
    /// The item being imported.
    pub name: Item,
    /// The item being imported.
    pub imported: Item,
    /// The module in which the imports is located.
    pub(crate) module: Rc<QueryMod>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum NameKind {
    Wildcard,
    Use,
    Other,
}

fn into_chain(chain: Vec<ImportEntryStep>) -> Vec<Location> {
    chain.into_iter().map(|c| c.location).collect()
}
