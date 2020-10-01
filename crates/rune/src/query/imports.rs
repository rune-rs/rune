use crate::collections::{HashMap, HashSet};
use crate::compiling::ImportEntryStep;
use crate::indexing::Visibility;
use crate::query::{QueryError, QueryErrorKind, QueryItem, QueryMod};
use crate::shared::Location;
use crate::{CompileError, Id};
use runestick::{Component, Item, Names, Span};
use std::fmt;
use std::rc::Rc;

/// Broken out substruct of `Query` to handle imports.
pub(crate) struct Imports {
    /// Prelude from the prelude.
    pub(super) prelude: HashMap<Box<str>, Item>,
    /// All imports in the current unit.
    ///
    /// Only used to link against the current environment to make sure all
    /// required units are present.
    pub(super) imports: HashMap<ImportKey, Rc<ImportEntry>>,
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
        was_imported: &mut bool,
    ) -> Result<Option<Item>, CompileError> {
        debug_assert!(base.starts_with(&mod_item.item));

        let mut base = base.clone();

        loop {
            let item = base.extended(local);

            if let Some((NameKind::Other, ..)) = self.names.get(&item) {
                return Ok(Some(item));
            }

            if let Some(item) = self.get_import(mod_item, spanned, &item)? {
                *was_imported = true;
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

        loop {
            let mut item = current.clone();

            let local = match item.pop() {
                Some(local) => local,
                None => return Ok(None),
            };

            let key = ImportKey {
                item,
                component: local,
            };

            if let Some(entry) = self.imports.get(&key).cloned() {
                if entry.mod_item.item != from.item {
                    match entry.visibility {
                        // TODO: make this more sophisticated.
                        Visibility::Public => (),
                        visibility => {
                            return Err(QueryError::new(
                                spanned,
                                QueryErrorKind::NotVisible {
                                    visibility,
                                    item: entry.item.clone(),
                                    from: from.item.clone(),
                                },
                            ));
                        }
                    }
                }

                // NB: if it's not in here, it's imported in the prelude.
                if let Some(item) = self.items_rev.get(&entry.item) {
                    self.check_access_from(spanned, &*from, item)?;
                    from = item.mod_item.clone();
                }

                // NB: this happens when you have a superflous import, like:
                // ```
                // use std;
                //
                // std::option::Option::None
                // ```
                if entry.item == current {
                    break;
                }

                path.push(ImportEntryStep {
                    location: entry.location,
                    visibility: entry.visibility,
                    item: entry.item.clone(),
                });

                if !visited.insert(entry.item.clone()) {
                    return Err(QueryError::new(
                        spanned,
                        QueryErrorKind::ImportCycle { path },
                    ));
                }

                matched = true;
                current = entry.item.clone();
                continue;
            }

            break;
        }

        if matched {
            return Ok(Some(current));
        }

        Ok(None)
    }

    /// Check that the given item is accessible from the given module.
    pub(crate) fn check_access_from(
        &self,
        spanned: Span,
        from: &QueryMod,
        item: &QueryItem,
    ) -> Result<(), QueryError> {
        let (mut mod_item, suffix, is_strict_prefix) =
            from.item.module_difference(&item.mod_item.item);

        // NB: if we are an immediate parent module, we're allowed to peek into
        // a nested private module in one level of depth.
        let mut permit_one_level = is_strict_prefix;
        let mut suffix_len = 0;

        for c in &suffix {
            suffix_len += 1;
            let permit_one_level = std::mem::take(&mut permit_one_level);

            mod_item.push(c);

            let m = self.modules.get(&mod_item).ok_or_else(|| {
                QueryError::new(
                    spanned,
                    QueryErrorKind::MissingMod {
                        item: mod_item.clone(),
                    },
                )
            })?;

            match m.visibility {
                Visibility::Public => (),
                Visibility::Crate => (),
                Visibility::Inherited => {
                    if !permit_one_level {
                        return Err(QueryError::new(
                            spanned,
                            QueryErrorKind::NotVisibleMod {
                                visibility: m.visibility,
                                item: mod_item,
                            },
                        ));
                    }
                }
            }
        }

        if suffix_len == 0 || is_strict_prefix && suffix_len > 1 {
            match item.visibility {
                Visibility::Inherited => {
                    if !from.item.can_see_private_mod(&item.mod_item.item) {
                        return Err(QueryError::new(
                            spanned,
                            QueryErrorKind::NotVisible {
                                visibility: item.visibility,
                                item: item.item.clone(),
                                from: from.item.clone(),
                            },
                        ));
                    }
                }
                Visibility::Public => (),
                Visibility::Crate => (),
            }
        }

        Ok(())
    }
}

/// The key of an import.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportKey {
    /// Where the import is located.
    pub item: Item,
    /// The component that is imported.
    pub component: Component,
}

impl fmt::Display for ImportKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.item, self.component)
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
    pub item: Item,
    /// The module in which the imports is located.
    pub(crate) mod_item: Rc<QueryMod>,
}

#[derive(Debug)]
pub(crate) enum NameKind {
    Use,
    Other,
}
