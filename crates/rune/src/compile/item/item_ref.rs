use core::fmt;

use smallvec::ToSmallVec;

use crate::compile::item::{Component, ComponentRef, IntoComponent, Item, Iter};

/// The reference to an [Item].
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ItemRef {
    pub(super) content: [u8],
}

impl ItemRef {
    /// Construct an [ItemRef] from an [Item].
    ///
    /// # Safety
    ///
    /// Caller must ensure that content has a valid [Item] representation.
    pub(super) unsafe fn new(content: &[u8]) -> &Self {
        &*(content as *const _ as *const _)
    }

    /// Get the crate corresponding to the item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// let item = Item::with_crate("std");
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let item = Item::with_item(&["local"]);
    /// assert_eq!(item.as_crate(), None);
    /// ```
    pub fn as_crate(&self) -> Option<&str> {
        if let Some(ComponentRef::Crate(s)) = self.iter().next() {
            Some(s)
        } else {
            None
        }
    }

    /// Access the first component of this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{ComponentRef, Item};
    ///
    /// let item = Item::with_item(&["foo", "bar"]);
    /// assert_eq!(item.first(), Some(ComponentRef::Str("foo")));
    /// ```
    pub fn first(&self) -> Option<ComponentRef<'_>> {
        self.iter().next()
    }

    /// Check if the item is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// let item = Item::new();
    /// assert!(item.is_empty());
    ///
    /// let item = Item::with_crate("std");
    /// assert!(!item.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Construct a new vector from the current item.
    pub fn as_vec(&self) -> Vec<Component> {
        self.iter()
            .map(ComponentRef::into_component)
            .collect::<Vec<_>>()
    }

    /// If the item only contains one element, return that element.
    pub fn as_local(&self) -> Option<&str> {
        let mut it = self.iter();

        match it.next_back_str() {
            Some(last) if it.is_empty() => Some(last),
            _ => None,
        }
    }

    /// Join this path with another.
    pub fn join<I>(&self, other: I) -> Item
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = self.content.to_smallvec();

        for c in other {
            c.write_component(&mut content);
        }

        // SAFETY: construction through write_component ensures valid
        // construction of buffer.
        unsafe { Item::from_raw(content) }
    }

    /// Clone and extend the item path.
    pub fn extended<C>(&self, part: C) -> Item
    where
        C: IntoComponent,
    {
        let mut content = self.content.to_smallvec();
        part.write_component(&mut content);

        // SAFETY: construction through write_component ensures valid
        // construction of buffer.
        unsafe { Item::from_raw(content) }
    }

    /// Access the last component in the path.
    pub fn last(&self) -> Option<ComponentRef<'_>> {
        self.iter().next_back()
    }

    /// An iterator over the [Component]s that constitute this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ComponentRef, IntoComponent};
    ///
    /// let mut item = Item::new();
    ///
    /// item.push("start");
    /// item.push(ComponentRef::Id(1));
    /// item.push(ComponentRef::Id(2));
    /// item.push("middle");
    /// item.push(ComponentRef::Id(3));
    /// item.push("end");
    ///
    /// let mut it = item.iter();
    ///
    /// assert_eq!(it.next(), Some("start".as_component_ref()));
    /// assert_eq!(it.next(), Some(ComponentRef::Id(1)));
    /// assert_eq!(it.next(), Some(ComponentRef::Id(2)));
    /// assert_eq!(it.next(), Some("middle".as_component_ref()));
    /// assert_eq!(it.next(), Some(ComponentRef::Id(3)));
    /// assert_eq!(it.next(), Some("end".as_component_ref()));
    /// assert_eq!(it.next(), None);
    ///
    /// assert!(!item.is_empty());
    /// ```
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(&self.content)
    }

    /// Test if current item starts with another.
    pub fn starts_with(&self, other: &Self) -> bool {
        self.content.starts_with(&other.content)
    }

    /// Test if current is immediate super of `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// assert!(Item::new().is_super_of(&Item::new(), 1));
    /// assert!(!Item::with_item(&["a"]).is_super_of(&Item::new(), 1));
    ///
    /// assert!(!Item::with_item(&["a", "b"]).is_super_of(&Item::with_item(&["a"]), 1));
    /// assert!(Item::with_item(&["a", "b"]).is_super_of(&Item::with_item(&["a", "b"]), 1));
    /// assert!(!Item::with_item(&["a"]).is_super_of(&Item::with_item(&["a", "b", "c"]), 1));
    /// ```
    pub fn is_super_of(&self, other: &Self, n: usize) -> bool {
        if self == other {
            return true;
        }

        let mut it = other.iter();

        for _ in 0..n {
            if it.next_back().is_none() {
                return false;
            }

            if self == it {
                return true;
            }
        }

        false
    }

    /// Get the ancestry of one module to another.
    ///
    /// This returns three things:
    /// * The shared prefix between the current and the `other` path.
    /// * The suffix to get to the `other` path from the shared prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// assert_eq!(
    ///     (Item::new(), Item::new()),
    ///     Item::new().ancestry(&Item::new())
    /// );
    ///
    /// assert_eq!(
    ///     (Item::new(), Item::with_item(&["a"])),
    ///     Item::new().ancestry(&Item::with_item(&["a"]))
    /// );
    ///
    /// assert_eq!(
    ///     (Item::new(), Item::with_item(&["a", "b"])),
    ///     Item::new().ancestry(&Item::with_item(&["a", "b"]))
    /// );
    ///
    /// assert_eq!(
    ///     (Item::with_item(&["a"]), Item::with_item(&["b"])),
    ///     Item::with_item(&["a", "c"]).ancestry(&Item::with_item(&["a", "b"]))
    /// );
    ///
    /// assert_eq!(
    ///     (Item::with_item(&["a", "b"]), Item::with_item(&["d", "e"])),
    ///     Item::with_item(&["a", "b", "c"]).ancestry(&Item::with_item(&["a", "b", "d", "e"]))
    /// );
    /// ```
    pub fn ancestry(&self, other: &Self) -> (Item, Item) {
        let mut a = self.iter();
        let mut b = other.iter();

        let mut shared = Item::new();
        let mut suffix = Item::new();

        while let Some(v) = b.next() {
            if let Some(u) = a.next() {
                if u == v {
                    shared.push(v);
                    continue;
                } else {
                    suffix.push(v);
                    suffix.extend(b);
                    return (shared, suffix);
                }
            }

            suffix.push(v);
            break;
        }

        suffix.extend(b);
        (shared, suffix)
    }

    /// Get the parent item for the current item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// let item = Item::with_item(&["foo", "bar", "baz"]);
    /// let item2 = Item::with_item(&["foo", "bar"]);
    ///
    /// assert_eq!(item.parent(), Some(&*item2));
    /// ```
    pub fn parent(&self) -> Option<&ItemRef> {
        let mut it = self.iter();
        it.next_back()?;
        Some(it.into_item())
    }
}

/// Format implementation for an [Item].
///
/// An empty item is formatted as `{root}`, because it refers to the topmost
/// root module.
///
/// # Examples
///
/// ```
/// use rune::compile::{Item, ComponentRef::*};
///
/// assert_eq!("{root}", Item::new().to_string());
/// assert_eq!("hello::$0", Item::with_item(&[Str("hello"), Id(0)]).to_string());
/// ```
impl fmt::Display for ItemRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Write;
        let mut it = self.iter();

        if let Some(last) = it.next_back() {
            let mut buf = String::new();

            for p in it {
                write!(buf, "{}::", p)?;
            }

            write!(buf, "{}", last)?;
            f.pad(&buf)
        } else {
            f.pad("{root}")
        }
    }
}

impl fmt::Debug for ItemRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Item({})", self)
    }
}

impl<'a> IntoIterator for &'a ItemRef {
    type IntoIter = Iter<'a>;
    type Item = ComponentRef<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq<Item> for ItemRef {
    fn eq(&self, other: &Item) -> bool {
        &self.content == other.content.as_ref()
    }
}

impl PartialEq<&Item> for ItemRef {
    fn eq(&self, other: &&Item) -> bool {
        &self.content == other.content.as_ref()
    }
}

impl PartialEq<Item> for &ItemRef {
    fn eq(&self, other: &Item) -> bool {
        &self.content == other.content.as_ref()
    }
}
