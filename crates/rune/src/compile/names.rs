use core::mem::replace;

use crate::alloc;
use crate::alloc::btree_map::{self, BTreeMap};
use crate::alloc::prelude::*;
use crate::compile::{Component, ComponentRef, IntoComponent};

/// A tree of names.
#[derive(Default, Debug)]
pub struct Names {
    root: Node,
}

impl TryClone for Names {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            root: self.root.try_clone()?,
        })
    }
}

impl Names {
    /// Insert the given item as an import.
    pub(crate) fn insert<I>(&mut self, iter: I) -> alloc::Result<bool>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
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
    pub(crate) fn contains<I>(&self, iter: I) -> alloc::Result<bool>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Ok(self.find_node(iter)?.map(|n| n.term).unwrap_or_default())
    }

    /// Test if we contain the given prefix.
    pub(crate) fn contains_prefix<I>(&self, iter: I) -> alloc::Result<bool>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Ok(self.find_node(iter)?.is_some())
    }

    /// Iterate over all known components immediately under the specified `iter`
    /// path.
    pub(crate) fn iter_components<'a, I: 'a>(
        &'a self,
        iter: I,
    ) -> alloc::Result<impl Iterator<Item = ComponentRef<'a>> + 'a>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let iter = if let Some(current) = self.find_node(iter)? {
            current.children.keys()
        } else {
            btree_map::Keys::default()
        };

        Ok(iter.map(|c| c.as_component_ref()))
    }

    /// Find the node corresponding to the given path.
    fn find_node<I>(&self, iter: I) -> alloc::Result<Option<&Node>>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
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

#[cfg(test)]
mod tests {
    use super::Names;
    use crate::support::Result;

    #[test]
    fn insert() -> Result<()> {
        let mut names = Names::default();
        assert!(!names.contains(["test"])?);
        assert!(!names.insert(["test"]).unwrap());
        assert!(names.contains(["test"])?);
        assert!(names.insert(["test"]).unwrap());
        Ok(())
    }

    #[test]
    fn contains() -> Result<()> {
        let mut names = Names::default();
        assert!(!names.contains(["test"])?);
        assert!(!names.insert(["test"]).unwrap());
        assert!(names.contains(["test"])?);
        assert!(names.insert(["test"]).unwrap());
        Ok(())
    }
}
