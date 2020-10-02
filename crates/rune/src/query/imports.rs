use crate::collections::HashMap;
use crate::indexing::Visibility;
use crate::query::{QueryError, QueryErrorKind, QueryItem, QueryMod};
use crate::shared::Location;
use crate::Id;
use runestick::{Item, Names, Span};
use std::rc::Rc;

/// Broken out substruct of `Query` to handle imports.
pub(crate) struct Imports {
    /// Prelude from the prelude.
    pub(super) prelude: HashMap<Box<str>, Item>,
    /// All imports in the current unit.
    ///
    /// Only used to link against the current environment to make sure all
    /// required units are present.
    pub(super) imports: HashMap<Item, Rc<ImportEntry>>,
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
        chain: &mut Vec<Location>,
        mod_item: &QueryMod,
        location: Location,
        visibility: Visibility,
        item: &Item,
    ) -> Result<(), QueryError> {
        let (common, tree) = from.item.ancestry(&mod_item.item);
        let mut module = common.clone();

        // Check each module from the common ancestrly to the module.
        for c in &tree {
            module.push(c);

            let m = self.modules.get(&module).ok_or_else(|| {
                QueryError::new(
                    spanned,
                    QueryErrorKind::MissingMod {
                        item: module.clone(),
                    },
                )
            })?;

            if !m.visibility.is_visible_to(&common, &module) {
                return Err(QueryError::new(
                    spanned,
                    QueryErrorKind::NotVisibleMod {
                        chain: std::mem::take(chain),
                        location: m.location,
                        visibility: m.visibility,
                        item: module,
                    },
                ));
            }
        }

        if !visibility.is_visible_to(&common, &mod_item.item) {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::NotVisible {
                    chain: std::mem::take(chain),
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
    pub(crate) mod_item: Rc<QueryMod>,
}

#[derive(Debug)]
pub(crate) enum NameKind {
    Use,
    Other,
}
