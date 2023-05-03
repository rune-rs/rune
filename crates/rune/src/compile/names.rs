use core::mem::replace;

use crate::collections::BTreeMap;
use crate::compile::{Component, ComponentRef, IntoComponent};

/// A tree of names.
#[derive(Default, Debug, Clone)]
pub struct Names {
    root: Node,
}

impl Names {
    /// Insert the given item as an import.
    pub(crate) fn insert<I>(&mut self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut current = &mut self.root;

        for c in iter {
            current = current.children.entry(c.into_component()).or_default();
        }

        replace(&mut current.term, true)
    }

    /// Test if the given import exists.
    pub(crate) fn contains<I>(&self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter).map(|n| n.term).unwrap_or_default()
    }

    /// Test if we contain the given prefix.
    pub(crate) fn contains_prefix<I>(&self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter).is_some()
    }

    /// Iterate over all known components immediately under the specified `iter`
    /// path.
    pub(crate) fn iter_components<'a, I: 'a>(
        &'a self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'a>> + 'a
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut current = &self.root;

        for c in iter {
            let c = c.into_component();

            current = match current.children.get(&c) {
                Some(node) => node,
                None => return IterComponents(None),
            };
        }

        return IterComponents(Some(current.children.keys()));

        struct IterComponents<I>(Option<I>);

        impl<'a, I> Iterator for IterComponents<I>
        where
            I: Iterator<Item = &'a Component>,
        {
            type Item = ComponentRef<'a>;

            fn next(&mut self) -> Option<Self::Item> {
                let mut iter = self.0.take()?;
                let next = iter.next()?;
                self.0 = Some(iter);
                Some(next.as_component_ref())
            }
        }
    }

    /// Find the node corresponding to the given path.
    fn find_node<I>(&self, iter: I) -> Option<&Node>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut current = &self.root;

        for c in iter {
            let c = c.as_component_ref().into_component();
            current = current.children.get(&c)?;
        }

        Some(current)
    }
}

#[derive(Default, Debug, Clone)]
struct Node {
    /// If the node is terminating.
    term: bool,
    /// The children of this node.
    children: BTreeMap<Component, Node>,
}

#[cfg(test)]
mod tests {
    use super::Names;

    #[test]
    fn insert() {
        let mut names = Names::default();
        assert!(!names.contains(["test"]));
        assert!(!names.insert(["test"]));
        assert!(names.contains(["test"]));
        assert!(names.insert(["test"]));
    }

    #[test]
    fn contains() {
        let mut names = Names::default();
        assert!(!names.contains(["test"]));
        assert!(!names.insert(["test"]));
        assert!(names.contains(["test"]));
        assert!(names.insert(["test"]));
    }
}
