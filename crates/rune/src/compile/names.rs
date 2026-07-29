#[cfg(test)]
mod tests;

use core::mem::{replace, take};

use crate::alloc;
use crate::alloc::btree_map::{self, BTreeMap};
use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::item::{Component, ComponentRef, IntoComponent};

/// A tree of names.
#[derive(Default, Debug)]
pub struct Names {
    root: Node,
}

impl TryClone for Names {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            root: self.root.try_clone()?,
        })
    }
}

impl Names {
    /// Insert the given item as an import.
    pub(crate) fn insert(
        &mut self,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> alloc::Result<bool> {
        let mut current = &mut self.root;

        for c in iter {
            current = current
                .children
                .entry(c.into_component()?)
                .or_try_default()?;
        }

        Ok(replace(&mut current.term, true))
    }

    /// Test if the given import exists.
    pub(crate) fn contains(
        &self,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> alloc::Result<bool> {
        Ok(self.find_node(iter)?.map(|n| n.term).unwrap_or_default())
    }

    /// Test if we contain the given prefix.
    pub(crate) fn contains_prefix(
        &self,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> alloc::Result<bool> {
        Ok(self.find_node(iter)?.is_some())
    }

    /// Iterate over all known components immediately under the specified `iter`
    /// path.
    pub(crate) fn iter_components<'a>(
        &'a self,
        iter: impl IntoIterator<Item: IntoComponent> + 'a,
    ) -> alloc::Result<impl Iterator<Item = ComponentRef<'a>> + 'a> {
        let iter = if let Some(current) = self.find_node(iter)? {
            current.children.keys()
        } else {
            btree_map::Keys::default()
        };

        Ok(iter.map(|c| c.as_component_ref()))
    }

    /// Find the node corresponding to the given path.
    fn find_node(
        &self,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> alloc::Result<Option<&Node>> {
        let mut current = &self.root;

        for c in iter {
            let c = c.into_component()?;

            let Some(c) = current.children.get(&c) else {
                return Ok(None);
            };

            current = c;
        }

        Ok(Some(current))
    }
}

#[derive(Default, Debug)]
struct Node {
    /// If the node is terminating.
    term: bool,
    /// The children of this node.
    children: BTreeMap<Component, Node>,
}

impl TryClone for Node {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            term: self.term,
            children: self.children.try_clone()?,
        })
    }
}

impl Drop for Node {
    /// Dismantle the subtree over an explicit stack.
    ///
    /// An item contributes one node per component, so nested items make a tree
    /// as deep as they nest and letting drop glue recurse through it overflows
    /// on input the compiler otherwise accepts. Nodes taken off the stack have
    /// already had their children moved out, so dropping them is shallow.
    ///
    /// If the stack cannot grow the remainder is left to drop glue, which is no
    /// worse than not having tried.
    fn drop(&mut self) {
        let mut stack = Vec::new();

        let mut node = take(&mut self.children);

        loop {
            for (_, child) in node {
                if stack.try_push(child).is_err() {
                    return;
                }
            }

            let Some(mut next) = stack.pop() else {
                break;
            };

            node = take(&mut next.children);
        }
    }
}
