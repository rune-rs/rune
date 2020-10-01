use crate::collections::HashMap;
use crate::{Component, ComponentRef, IntoComponent};
use std::mem;

/// A tree of names.
#[derive(Debug)]
pub struct Names<T> {
    root: Node<T>,
}

impl<T> Default for Names<T> {
    fn default() -> Self {
        Names {
            root: Default::default(),
        }
    }
}

impl<T> Names<T> {
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
    /// let mut names = Names::<()>::new();
    /// assert!(!names.contains(&["test"]));
    /// assert!(names.insert(&["test"], ()).is_none());
    /// assert!(names.contains(&["test"]));
    /// assert!(names.insert(&["test"], ()).is_some());
    /// ```
    pub fn insert<I>(&mut self, iter: I, value: T) -> Option<T>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut current = &mut self.root;

        for c in iter {
            current = current.children.entry(c.into_component()).or_default();
        }

        mem::replace(&mut current.term, Some(value))
    }

    /// Test if the given import exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Names;
    ///
    /// let mut names = Names::<()>::new();
    /// assert!(!names.contains(&["test"]));
    /// assert!(names.insert(&["test"], ()).is_none());
    /// assert!(names.contains(&["test"]));
    /// assert!(names.insert(&["test"], ()).is_some());
    /// ```
    pub fn contains<I>(&self, iter: I) -> bool
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter)
            .map(|n| n.term.is_some())
            .unwrap_or_default()
    }

    /// Get the given entry if it exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Names;
    ///
    /// let mut names = Names::<()>::new();
    /// assert!(names.get(&["test"]).is_none());
    /// assert!(names.insert(&["test"], ()).is_none());
    /// assert!(names.get(&["test"]).is_some());
    /// assert!(names.insert(&["test"], ()).is_some());
    /// ```
    pub fn get<I>(&self, iter: I) -> Option<&T>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.find_node(iter).and_then(|n| n.term.as_ref())
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
    pub fn iter_components<'a, I: 'a>(
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
    fn find_node<I>(&self, iter: I) -> Option<&Node<T>>
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

#[derive(Debug)]
struct Node<T> {
    /// If this is a terminating node that can be imported or not..
    term: Option<T>,
    /// The children of this node.
    children: HashMap<Component, Node<T>>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            term: Default::default(),
            children: Default::default(),
        }
    }
}
