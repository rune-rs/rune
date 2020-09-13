use crate::collections::HashMap;
use crate::{Component, IntoComponent};
use std::mem;

#[derive(Default, Debug)]
struct Node {
    /// If this is a terminating node that can be imported or not..
    term: bool,
    /// The children of this node.
    children: HashMap<Component, Node>,
}

/// A tree of names.
#[derive(Default, Debug)]
pub struct Names {
    root: Node,
}

impl Names {
    /// Construct a collection of names.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert the given item as an import.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Names;
    ///
    /// let mut names = Names::new();
    /// assert!(!names.contains(&["test"]));
    /// assert!(names.insert(&["test"]));
    /// assert!(names.contains(&["test"]));
    /// assert!(!names.insert(&["test"]));
    /// ```
    pub fn insert<I>(&mut self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut current = &mut self.root;

        for c in iter {
            current = current.children.entry(c.into_component()).or_default();
        }

        !mem::replace(&mut current.term, true)
    }

    /// Test if the given import exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Names;
    ///
    /// let mut names = Names::new();
    /// assert!(!names.contains(&["test"]));
    /// assert!(names.insert(&["test"]));
    /// assert!(names.contains(&["test"]));
    /// assert!(!names.insert(&["test"]));
    /// ```
    pub fn contains<I>(&self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter).map(|n| n.term).unwrap_or_default()
    }

    /// Test if we contain the given prefix.
    pub fn contains_prefix<I>(&self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter).is_some()
    }

    /// Iterate over all known components immediately under the specified `iter`
    /// path.
    pub fn iter_components<'a, I: 'a>(&'a self, iter: I) -> impl Iterator<Item = Component> + 'a
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
            type Item = Component;

            fn next(&mut self) -> Option<Self::Item> {
                let mut iter = self.0.take()?;
                let next = iter.next()?;
                self.0 = Some(iter);
                Some(next.clone())
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
            let c = c.into_component();
            current = current.children.get(&c)?;
        }

        Some(current)
    }
}
