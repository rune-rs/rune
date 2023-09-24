use core::fmt;

#[cfg(feature = "alloc")]
use crate::alloc::borrow::TryToOwned;
#[cfg(feature = "alloc")]
use crate::alloc::iter::IteratorExt;
#[cfg(feature = "alloc")]
use crate::alloc::{self, Vec};

#[cfg(feature = "alloc")]
use crate::item::Component;
use crate::item::{ComponentRef, IntoComponent, ItemBuf, Iter};

/// The reference to an [ItemBuf].
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Item {
    content: [u8],
}

impl Item {
    /// Construct an [Item] corresponding to the root item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ItemBuf};
    ///
    /// assert_eq!(Item::new(), &*ItemBuf::new());
    /// ```
    #[inline]
    pub const fn new() -> &'static Self {
        // SAFETY: an empty slice is a valid bit pattern for the root.
        unsafe { Self::from_raw(&[]) }
    }

    /// Construct an [Item] from an [ItemBuf].
    ///
    /// # Safety
    ///
    /// Caller must ensure that content has a valid [ItemBuf] representation.
    pub(super) const unsafe fn from_raw(content: &[u8]) -> &Self {
        &*(content as *const _ as *const _)
    }

    /// Return the underlying byte representation of the [Item].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ItemBuf};
    ///
    /// assert_eq!(Item::new().as_bytes(), b"");
    ///
    /// let item = ItemBuf::with_item(["foo", "bar"])?;
    /// assert_eq!(item.as_bytes(), b"\x0d\0foo\x0d\0\x0d\0bar\x0d\0");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.content
    }

    /// Get the crate corresponding to the item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::ItemBuf;
    ///
    /// let item = ItemBuf::with_crate("std")?;
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let item = ItemBuf::with_item(["local"])?;
    /// assert_eq!(item.as_crate(), None);
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::compile::{ComponentRef, ItemBuf};
    ///
    /// let item = ItemBuf::with_item(["foo", "bar"])?;
    /// assert_eq!(item.first(), Some(ComponentRef::Str("foo")));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn first(&self) -> Option<ComponentRef<'_>> {
        self.iter().next()
    }

    /// Check if the item is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::ItemBuf;
    ///
    /// let item = ItemBuf::new();
    /// assert!(item.is_empty());
    ///
    /// let item = ItemBuf::with_crate("std")?;
    /// assert!(!item.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Construct a new vector from the current item.
    #[cfg(feature = "alloc")]
    pub fn as_vec(&self) -> alloc::Result<Vec<Component>> {
        self.iter()
            .map(ComponentRef::into_component)
            .try_collect::<Result<Vec<_>, _>>()?
    }

    /// If the item only contains one element, return that element.
    pub fn as_local(&self) -> Option<&str> {
        let mut it = self.iter();

        match it.next_back_str() {
            Some(last) if it.is_empty() => Some(last),
            _ => None,
        }
    }

    /// Return an owned and joined variant of this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ComponentRef};
    ///
    /// let item = Item::new();
    /// assert!(item.is_empty());
    ///
    /// let item2 = item.join(["hello", "world"])?;
    /// assert_eq!(item2.first(), Some(ComponentRef::Str("hello")));
    /// assert_eq!(item2.last(), Some(ComponentRef::Str("world")));
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn join<I>(&self, other: I) -> alloc::Result<ItemBuf>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = self.content.try_to_owned()?;

        for c in other {
            c.write_component(&mut content)?;
        }

        // SAFETY: construction through write_component ensures valid
        // construction of buffer.
        Ok(unsafe { ItemBuf::from_raw(content) })
    }

    /// Return an owned and extended variant of this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ComponentRef};
    ///
    /// let item = Item::new();
    /// assert!(item.is_empty());
    ///
    /// let item2 = item.extended("hello")?;
    /// assert_eq!(item2.first(), Some(ComponentRef::Str("hello")));
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn extended<C>(&self, part: C) -> alloc::Result<ItemBuf>
    where
        C: IntoComponent,
    {
        let mut content = self.content.try_to_owned()?;
        part.write_component(&mut content)?;

        // SAFETY: construction through write_component ensures valid
        // construction of buffer.
        Ok(unsafe { ItemBuf::from_raw(content) })
    }

    /// Access the last component in the path.
    #[inline]
    pub fn last(&self) -> Option<ComponentRef<'_>> {
        self.iter().next_back()
    }

    /// An iterator over the [Component]s that constitute this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{ComponentRef, IntoComponent, ItemBuf};
    ///
    /// let mut item = ItemBuf::new();
    ///
    /// item.push("start")?;
    /// item.push(ComponentRef::Id(1))?;
    /// item.push(ComponentRef::Id(2))?;
    /// item.push("middle")?;
    /// item.push(ComponentRef::Id(3))?;
    /// item.push("end")?;
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
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(&self.content)
    }

    /// Test if current item starts with another.
    #[inline]
    pub fn starts_with<U>(&self, other: U) -> bool
    where
        U: AsRef<Item>,
    {
        self.content.starts_with(&other.as_ref().content)
    }

    /// Test if current is immediate super of `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::{Item, ItemBuf};
    ///
    /// assert!(Item::new().is_super_of(Item::new(), 1));
    /// assert!(!ItemBuf::with_item(["a"])?.is_super_of(Item::new(), 1));
    ///
    /// assert!(!ItemBuf::with_item(["a", "b"])?.is_super_of(ItemBuf::with_item(["a"])?, 1));
    /// assert!(ItemBuf::with_item(["a", "b"])?.is_super_of(ItemBuf::with_item(["a", "b"])?, 1));
    /// assert!(!ItemBuf::with_item(["a"])?.is_super_of(ItemBuf::with_item(["a", "b", "c"])?, 1));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn is_super_of<U>(&self, other: U, n: usize) -> bool
    where
        U: AsRef<Item>,
    {
        let other = other.as_ref();

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
    /// use rune::compile::{Item, ItemBuf};
    ///
    /// assert_eq!(
    ///     (ItemBuf::new(), ItemBuf::new()),
    ///     Item::new().ancestry(Item::new())?
    /// );
    ///
    /// assert_eq!(
    ///     (ItemBuf::new(), ItemBuf::with_item(["a"])?),
    ///     Item::new().ancestry(ItemBuf::with_item(["a"])?)?
    /// );
    ///
    /// assert_eq!(
    ///     (ItemBuf::new(), ItemBuf::with_item(["a", "b"])?),
    ///     Item::new().ancestry(ItemBuf::with_item(["a", "b"])?)?
    /// );
    ///
    /// assert_eq!(
    ///     (ItemBuf::with_item(["a"])?, ItemBuf::with_item(["b"])?),
    ///     ItemBuf::with_item(["a", "c"])?.ancestry(ItemBuf::with_item(["a", "b"])?)?
    /// );
    ///
    /// assert_eq!(
    ///     (ItemBuf::with_item(["a", "b"])?, ItemBuf::with_item(["d", "e"])?),
    ///     ItemBuf::with_item(["a", "b", "c"])?.ancestry(ItemBuf::with_item(["a", "b", "d", "e"])?)?
    /// );
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn ancestry<U>(&self, other: U) -> alloc::Result<(ItemBuf, ItemBuf)>
    where
        U: AsRef<Item>,
    {
        let mut a = self.iter();
        let other = other.as_ref();
        let mut b = other.iter();

        let mut shared = ItemBuf::new();
        let mut suffix = ItemBuf::new();

        while let Some(v) = b.next() {
            if let Some(u) = a.next() {
                if u == v {
                    shared.push(v)?;
                    continue;
                } else {
                    suffix.push(v)?;
                    suffix.extend(b)?;
                    return Ok((shared, suffix));
                }
            }

            suffix.push(v)?;
            break;
        }

        suffix.extend(b)?;
        Ok((shared, suffix))
    }

    /// Get the parent item for the current item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::compile::ItemBuf;
    ///
    /// let item = ItemBuf::with_item(["foo", "bar", "baz"])?;
    /// let item2 = ItemBuf::with_item(["foo", "bar"])?;
    ///
    /// assert_eq!(item.parent(), Some(&*item2));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn parent(&self) -> Option<&Item> {
        let mut it = self.iter();
        it.next_back()?;
        Some(it.into_item())
    }
}

impl AsRef<Item> for &Item {
    #[inline]
    fn as_ref(&self) -> &Item {
        self
    }
}

impl Default for &Item {
    #[inline]
    fn default() -> Self {
        Item::new()
    }
}

#[cfg(feature = "alloc")]
impl TryToOwned for Item {
    type Owned = ItemBuf;

    #[inline]
    fn try_to_owned(&self) -> alloc::Result<Self::Owned> {
        // SAFETY: item ensures that content is valid.
        Ok(unsafe { ItemBuf::from_raw(self.content.try_to_owned()?) })
    }
}

/// Format implementation for an [ItemBuf].
///
/// An empty item is formatted as `{root}`, because it refers to the topmost
/// root module.
///
/// # Examples
///
/// ```
/// use rune::compile::{ComponentRef, ItemBuf};
/// use rune::alloc::prelude::*;
///
/// let root = ItemBuf::new().try_to_string()?;
/// assert_eq!("{root}", root);
///
/// let hello = ItemBuf::with_item(&[ComponentRef::Str("hello"), ComponentRef::Id(0)])?;
/// assert_eq!("hello::$0", hello.try_to_string()?);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.iter();

        if let Some(last) = it.next_back() {
            for p in it {
                write!(f, "{}::", p)?;
            }

            write!(f, "{}", last)?;
        } else {
            f.write_str("{root}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'a> IntoIterator for &'a Item {
    type IntoIter = Iter<'a>;
    type Item = ComponentRef<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq<ItemBuf> for Item {
    fn eq(&self, other: &ItemBuf) -> bool {
        self.content == other.content
    }
}

impl PartialEq<ItemBuf> for &Item {
    fn eq(&self, other: &ItemBuf) -> bool {
        self.content == other.content
    }
}

impl PartialEq<Iter<'_>> for Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        self == other.as_item()
    }
}

impl PartialEq<Iter<'_>> for &Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        *self == other.as_item()
    }
}
