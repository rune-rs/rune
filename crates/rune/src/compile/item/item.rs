use core::fmt;
use core::hash::Hash;
use core::ops::Deref;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::compile::item::internal::INLINE;
use crate::compile::item::{Component, ComponentRef, IntoComponent, ItemRef, Iter};

/// The name of an item in the Rune Language.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
///
/// An item can also belongs to a crate, which in rune could be indicated as
/// `::crate::foo::bar`. These items must be constructed using
/// [Item::with_crate].
///
/// Items are inlined if they are smaller than 32 bytes.
///
/// # Panics
///
/// The max length of a string component is is 2**15 = 32768. Attempting to add
/// a string larger than that will panic.
///
/// # Component encoding
///
/// A component is encoded as:
/// * A two byte tag as a u16 in native endianess, indicating its type (least
///   significant 2 bits) and data (most significant 15 bits).
/// * If the type is a `STRING`, the data is treated as the length of the
///   string. Any other type this the `data` is treated as the numeric id of the
///   component.
/// * If the type is a `STRING`, the tag is repeated at the end of it to allow
///   for seeking backwards. This is **not** the case for other types. Since
///   they are fixed size its not necessary.
///
/// So all in all, a string is encoded as:
///
/// ```text
/// dddddddd dddddddt *string content* dddddddd dddddddt
/// ```
///
/// And any other component is just the two bytes:
///
/// ```text
/// dddddddd dddddddt
/// ```
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Item {
    content: SmallVec<[u8; INLINE]>,
}

impl Item {
    /// Construct a new empty item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::Item;
    ///
    /// let item = Item::new();
    /// let mut it = item.iter();
    ///
    /// assert_eq!(it.next(), None);
    /// ```
    pub const fn new() -> Self {
        Self {
            content: SmallVec::new_const(),
        }
    }

    /// Internal raw constructor for an item.
    ///
    /// # Safety
    ///
    /// Caller must ensure that its representation is valid.
    pub(super) const unsafe fn from_raw(content: SmallVec<[u8; INLINE]>) -> Self {
        Self { content }
    }

    /// Construct a new item with the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{ComponentRef, Item};
    ///
    /// let item = Item::with_item(&["foo", "bar"]);
    /// let mut it = item.iter();
    ///
    /// assert_eq!(it.next(), Some(ComponentRef::Str("foo")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("bar")));
    /// assert_eq!(it.next(), None);
    /// ```
    pub fn with_item<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = SmallVec::new();

        for c in iter {
            c.write_component(&mut content);
        }

        Self { content }
    }

    /// Construct item for a crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{ComponentRef, Item};
    ///
    /// let mut item = Item::with_crate("std");
    /// item.push("foo");
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let mut it = item.iter();
    /// assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("foo")));
    /// assert_eq!(it.next(), None);
    /// ```
    pub fn with_crate(name: &str) -> Self {
        Self::with_item(&[ComponentRef::Crate(name)])
    }

    /// Create a crated item with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{ComponentRef, Item};
    ///
    /// let item = Item::with_crate_item("std", &["option"]);
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let mut it = item.iter();
    /// assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("option")));
    /// assert_eq!(it.next(), None);
    /// ```
    pub fn with_crate_item<I>(name: &str, iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = SmallVec::new();
        ComponentRef::Crate(name).write_component(&mut content);

        for c in iter {
            c.write_component(&mut content);
        }

        Self { content }
    }

    /// Push the given component to the current item.
    pub fn push<C>(&mut self, c: C)
    where
        C: IntoComponent,
    {
        c.write_component(&mut self.content);
    }

    /// Push the given component to the current item.
    pub fn pop(&mut self) -> Option<Component> {
        let mut it = self.iter();
        let c = it.next_back()?.into_component();
        let new_len = it.len();
        self.content.resize(new_len, 0);
        Some(c)
    }

    /// Extend the current item with an iterator.
    pub fn extend<I>(&mut self, i: I)
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        for c in i {
            self.push(c);
        }
    }

    /// Clear the current item.
    pub fn clear(&mut self) {
        self.content.clear();
    }

    /// Convert into a vector from the current item.
    pub fn into_vec(self) -> Vec<Component> {
        self.into_iter().collect::<Vec<_>>()
    }
}

impl Deref for Item {
    type Target = ItemRef;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Item ensures that content is valid.
        unsafe { ItemRef::new(self.content.as_ref()) }
    }
}

/// Format implementation for an [Item], defers to [ItemRef].
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ItemRef::fmt(self, f)
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ItemRef::fmt(self, f)
    }
}

impl IntoIterator for Item {
    type IntoIter = std::vec::IntoIter<Component>;
    type Item = Component;

    fn into_iter(self) -> Self::IntoIter {
        self.as_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a Item {
    type IntoIter = Iter<'a>;
    type Item = ComponentRef<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq<ItemRef> for Item {
    fn eq(&self, other: &ItemRef) -> bool {
        self.content.as_ref() == &other.content
    }
}

impl PartialEq<ItemRef> for &Item {
    fn eq(&self, other: &ItemRef) -> bool {
        self.content.as_ref() == &other.content
    }
}

impl PartialEq<&ItemRef> for Item {
    fn eq(&self, other: &&ItemRef) -> bool {
        self.content.as_ref() == &other.content
    }
}
