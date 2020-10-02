use crate::collections::{HashMap, HashSet};
use crate::compiling::ImportEntryStep;
use crate::indexing::Visibility;
use crate::query::{QueryError, QueryErrorKind, QueryItem, QueryMod};
use crate::shared::Location;
use crate::{CompileError, Id};
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
    /// Walk the names to find the first one that is contained in the unit.
    pub(crate) fn walk_names(
        &mut self,
        spanned: Span,
        mod_item: &Rc<QueryMod>,
        base: &Item,
        local: &str,
    ) -> Result<Option<Item>, CompileError> {
        debug_assert!(base.starts_with(&mod_item.item));

        let mut base = base.clone();

        loop {
            let item = base.extended(local);

            if let Some((NameKind::Other, ..)) = self.names.get(&item) {
                return Ok(Some(item));
            }

            if let Some(item) = self.get_import(mod_item, spanned, &item)? {
                return Ok(Some(item));
            }

            if mod_item.item == base || base.pop().is_none() {
                break;
            }
        }

        if let Some(item) = self.prelude.get(local) {
            return Ok(Some(item.clone()));
        }

        Ok(None)
    }

    /// Get the given import by name.
    pub(crate) fn get_import(
        &mut self,
        mod_item: &Rc<QueryMod>,
        spanned: Span,
        item: &Item,
    ) -> Result<Option<Item>, QueryError> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        let mut current = item.clone();
        let mut matched = false;
        let mut from = mod_item.clone();
        let mut chain = Vec::new();

        loop {
            if let Some(entry) = self.imports.get(&current).cloned() {
                self.check_access_to(
                    spanned,
                    &*from,
                    &mut chain,
                    &entry.mod_item,
                    entry.location,
                    entry.visibility,
                    &entry.name,
                )?;

                from = entry.mod_item.clone();
                chain.push(entry.location);

                // NB: this happens when you have a superflous import, like:
                // ```
                // use std;
                //
                // std::option::Option::None
                // ```
                if entry.imported == current {
                    break;
                }

                path.push(ImportEntryStep {
                    location: entry.location,
                    visibility: entry.visibility,
                    item: entry.name.clone(),
                });

                if !visited.insert(entry.imported.clone()) {
                    return Err(QueryError::new(
                        spanned,
                        QueryErrorKind::ImportCycle { path },
                    ));
                }

                matched = true;
                current = entry.imported.clone();
                continue;
            }

            break;
        }

        if let Some(item) = self.items_rev.get(&current) {
            self.check_access_to(
                spanned,
                &*from,
                &mut chain,
                &item.mod_item,
                item.location,
                item.visibility,
                &item.item,
            )?;
        }

        if matched {
            return Ok(Some(current));
        }

        Ok(None)
    }

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
