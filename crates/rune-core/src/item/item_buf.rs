use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::mem::take;
use core::ops::Deref;
use core::str::FromStr;

use crate::alloc::alloc::{Allocator, Global};
use crate::alloc::clone::TryClone;
use crate::alloc::iter::TryFromIteratorIn;
use crate::alloc::{self, Vec};

use crate::item::{ComponentRef, IntoComponent, Item, Iter};

/// The name of an item in the Rune Language.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
///
/// An item can also belongs to a crate, which in rune could be indicated as
/// `::crate::foo::bar`. These items must be constructed using
/// [ItemBuf::with_crate].
///
/// Items are inlined if they are smaller than 32 bytes.
///
/// # Panics
///
/// The max length of a string component is is 2**14 = 16384. Attempting to add
/// a string larger than that will panic. This also constitutes the maximum
/// number of *nested* sibling components that can exist in a single source file
/// since they all use anonymous identifiers.
///
/// # Component encoding
///
/// The following details internal implementation details of an [`Item`], and is
/// not exposed through its API. It is provided here in case you need to work
/// with the internal of an item.
///
/// A single component is encoded as:
///
/// * A two byte tag as a u16 in native endianess, indicating its type (least
///   significant 2 bits) and data (most significant 14 bits).
/// * If the type is a `STRING`, the data is treated as the length of the
///   string. Any other type this the `data` is treated as the numeric id of the
///   component.
/// * If the type is a `STRING`, the tag is repeated at the end of it to allow
///   for seeking backwards. This is *not* the case for other types. Since they
///   are fixed size its not necessary.
///
/// So all in all, a string is encoded as this where the `d` part indicates the
/// length of the string:
///
/// ```text
/// dddddddd ddddddtt *string content* dddddddd ddddddtt
/// ```
///
/// And any other component is just the two bytes where the `d` part makes up a
/// numerical component:
///
/// ```text
/// dddddddd ddddddtt
/// ```
#[repr(transparent)]
pub struct ItemBuf<A = Global>
where
    A: Allocator,
{
    content: Vec<u8, A>,
}

impl<A> ItemBuf<A>
where
    A: Allocator,
{
    /// Construct a new item buffer inside of the given allocator.
    #[inline]
    pub(crate) const fn new_in(alloc: A) -> Self {
        Self {
            content: Vec::new_in(alloc),
        }
    }

    /// Internal raw constructor for an item.
    ///
    /// # Safety
    ///
    /// Caller must ensure that its representation is valid.
    #[inline]
    pub(super) const unsafe fn from_raw(content: Vec<u8, A>) -> Self {
        Self { content }
    }

    /// Construct a new item with the given path in the given allocator.
    #[inline]
    pub(crate) fn with_item_in(
        iter: impl IntoIterator<Item: IntoComponent>,
        alloc: A,
    ) -> alloc::Result<Self> {
        let mut content = Vec::new_in(alloc);

        for c in iter {
            c.write_component(&mut content)?;
        }

        Ok(Self { content })
    }

    /// Push the given component to the current item.
    #[inline]
    pub fn push(&mut self, c: impl IntoComponent) -> alloc::Result<()> {
        c.write_component(&mut self.content)?;
        Ok(())
    }

    /// Pop a the tail component, returning `true` if there was something to pop.
    pub fn pop(&mut self) -> bool {
        let mut it = self.iter();

        if it.next_back().is_none() {
            return false;
        };

        let new_len = it.len();

        // SAFETY: Advancing the back end of the iterator ensures that the new
        // length is smaller than the original, and an item buffer is a byte
        // array which does not need to be dropped.
        unsafe {
            debug_assert!(new_len < self.content.len());
            self.content.set_len(new_len);
        }

        true
    }

    /// Extend the current item with an iterator.
    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item: IntoComponent>) -> alloc::Result<()> {
        for c in iter {
            self.push(c)?;
        }

        Ok(())
    }

    /// Clear the current item.
    #[inline]
    pub fn clear(&mut self) {
        self.content.clear();
    }
}

impl ItemBuf {
    /// Construct a new empty item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ItemBuf;
    ///
    /// let item = ItemBuf::new();
    /// let mut it = item.iter();
    ///
    /// assert_eq!(it.next(), None);
    /// ```
    pub const fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// Construct a new item with the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ItemBuf;
    /// use rune::item::ComponentRef;
    ///
    /// let item = ItemBuf::with_item(["foo", "bar"])?;
    /// let mut it = item.iter();
    ///
    /// assert_eq!(it.next(), Some(ComponentRef::Str("foo")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("bar")));
    /// assert_eq!(it.next(), None);
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn with_item(iter: impl IntoIterator<Item: IntoComponent>) -> alloc::Result<Self> {
        Self::with_item_in(iter, Global)
    }

    /// Construct item for a crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ItemBuf;
    /// use rune::item::ComponentRef;
    ///
    /// let mut item = ItemBuf::with_crate("std")?;
    /// item.push("foo");
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let mut it = item.iter();
    /// assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("foo")));
    /// assert_eq!(it.next(), None);
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn with_crate(name: &str) -> alloc::Result<Self> {
        Self::with_item(&[ComponentRef::Crate(name)])
    }

    /// Create a crated item with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ItemBuf;
    /// use rune::item::ComponentRef;
    ///
    /// let item = ItemBuf::with_crate_item("std", ["option"])?;
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let mut it = item.iter();
    /// assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
    /// assert_eq!(it.next(), Some(ComponentRef::Str("option")));
    /// assert_eq!(it.next(), None);
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    #[inline]
    pub fn with_crate_item(
        name: &str,
        iter: impl IntoIterator<Item: IntoComponent>,
    ) -> alloc::Result<Self> {
        let mut content = Vec::new();
        ComponentRef::Crate(name).write_component(&mut content)?;

        for c in iter {
            c.write_component(&mut content)?;
        }

        Ok(Self { content })
    }
}

impl<A> Default for ItemBuf<A>
where
    A: Allocator + Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            content: Vec::new_in(A::default()),
        }
    }
}

impl<A> PartialEq for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}

impl<A> Eq for ItemBuf<A> where A: Allocator {}

impl<A> PartialOrd for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.content.cmp(&other.content))
    }
}

impl<A> Ord for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.content.cmp(&other.content)
    }
}

impl<A> Hash for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.content.hash(state);
    }
}

impl<A> TryClone for ItemBuf<A>
where
    A: Allocator + Clone,
{
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            content: self.content.try_clone()?,
        })
    }
}

impl<A> AsRef<Item> for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn as_ref(&self) -> &Item {
        self
    }
}

impl<A> Borrow<Item> for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn borrow(&self) -> &Item {
        self
    }
}

impl<C, A> TryFromIteratorIn<C, A> for ItemBuf<A>
where
    C: IntoComponent,
    A: Allocator,
{
    #[inline]
    fn try_from_iter_in<T: IntoIterator<Item = C>>(iter: T, alloc: A) -> alloc::Result<Self> {
        Self::with_item_in(iter, alloc)
    }
}

impl<A> Deref for ItemBuf<A>
where
    A: Allocator,
{
    type Target = Item;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: Item ensures that content is valid.
        unsafe { Item::from_bytes(self.content.as_ref()) }
    }
}

/// Format implementation for an [ItemBuf], defers to [Item].
impl<A> fmt::Display for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Item::fmt(self, f)
    }
}

impl<A> fmt::Debug for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Item::fmt(self, f)
    }
}

impl<'a, A> IntoIterator for &'a ItemBuf<A>
where
    A: Allocator,
{
    type IntoIter = Iter<'a>;
    type Item = ComponentRef<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<A> PartialEq<Item> for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &Item) -> bool {
        self.content.as_slice() == other.as_bytes()
    }
}

impl<A> PartialEq<Item> for &ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &Item) -> bool {
        self.content.as_slice() == other.as_bytes()
    }
}

impl<A> PartialEq<&Item> for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &&Item) -> bool {
        self.content.as_slice() == other.as_bytes()
    }
}

impl<A> PartialEq<Iter<'_>> for ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &Iter<'_>) -> bool {
        self == other.as_item()
    }
}

impl<A> PartialEq<Iter<'_>> for &ItemBuf<A>
where
    A: Allocator,
{
    #[inline]
    fn eq(&self, other: &Iter<'_>) -> bool {
        *self == other.as_item()
    }
}

/// Error when parsing an item.
#[derive(Debug)]
#[non_exhaustive]
pub struct FromStrError {
    kind: FromStrErrorKind,
}

impl From<alloc::Error> for FromStrError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self {
            kind: FromStrErrorKind::AllocError(error),
        }
    }
}

impl From<FromStrErrorKind> for FromStrError {
    #[inline]
    fn from(kind: FromStrErrorKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug)]
enum FromStrErrorKind {
    /// Error during parse.
    ParseError,
    /// An error occured when allocating.
    AllocError(alloc::Error),
}

impl fmt::Display for FromStrError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FromStrErrorKind::ParseError => write!(f, "String is not a valid item"),
            FromStrErrorKind::AllocError(error) => error.fmt(f),
        }
    }
}

impl core::error::Error for FromStrError {}

impl<A> FromStr for ItemBuf<A>
where
    A: Allocator + Default,
{
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut item = ItemBuf::new_in(A::default());

        let (s, mut next_crate) = if let Some(remainder) = s.strip_prefix("::") {
            (remainder, true)
        } else {
            (s, false)
        };

        for c in s.split("::") {
            if take(&mut next_crate) {
                item.push(ComponentRef::Crate(c))?;
            } else if let Some(num) = c.strip_prefix('$') {
                item.push(ComponentRef::Id(
                    num.parse().map_err(|_| FromStrErrorKind::ParseError)?,
                ))?;
            } else {
                item.push(ComponentRef::Str(c))?;
            }
        }

        Ok(item)
    }
}
