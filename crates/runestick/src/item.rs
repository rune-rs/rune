use crate::RawStr;
use byteorder::{ByteOrder as _, NativeEndian, WriteBytesExt as _};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom as _;
use std::fmt;
use std::hash;
use std::hash::Hash as _;

// Types available.
const CRATE: u8 = 0;
const STRING: u8 = 1;
const ID: u8 = 2;

/// How many bits the type of a tag takes up.
const TYPE_BITS: usize = 2;
/// Mask of the type of a tag.
const TYPE_MASK: usize = (0b1 << TYPE_BITS) - 1;
/// Total tag size in bytes.
const TAG_BYTES: usize = 2;
/// Max size of data stored.
const MAX_DATA: usize = 0b1 << (TAG_BYTES * 8 - TYPE_BITS);

/// The name of an item.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
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
pub struct Item {
    content: Vec<u8>,
}

impl Item {
    /// Construct an empty item.
    pub const fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// Construct a new item path.
    pub fn with_item<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = Vec::new();

        for c in iter {
            c.write_component(&mut content);
        }

        Self { content }
    }

    /// Construct item for a crate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::{Item, ComponentRef};
    ///
    /// let item = Item::with_crate("std");
    /// assert_eq!(item.as_crate(), Some("std"));
    ///
    /// let mut it = item.iter();
    /// assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
    /// assert_eq!(it.next(), None);
    /// ```
    pub fn with_crate(name: &str) -> Self {
        Self::with_item(&[ComponentRef::Crate(name)])
    }

    /// Create a crated item with the given name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::{Item, ComponentRef};
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
        let mut content = Vec::new();
        ComponentRef::Crate(name).write_component(&mut content);

        for c in iter {
            c.write_component(&mut content);
        }

        Self { content }
    }

    /// Get the crate corresponding to the item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Item;
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
    /// ```rust
    /// use runestick::{ComponentRef, Item};
    ///
    /// let item = Item::with_item(&["foo", "bar"]);
    /// assert_eq!(item.first(), Some(ComponentRef::Str("foo")));
    /// ```
    pub fn first(&self) -> Option<ComponentRef<'_>> {
        self.iter().next()
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
        let new_len = it.content.len();
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

    /// Check if the item is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Item;
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

    /// Clear the current item.
    pub fn clear(&mut self) {
        self.content.clear();
    }

    /// Construct a new vector from the current item.
    pub fn as_vec(&self) -> Vec<Component> {
        self.iter()
            .map(ComponentRef::into_component)
            .collect::<Vec<_>>()
    }

    /// Convert into a vector from the current item.
    pub fn into_vec(self) -> Vec<Component> {
        self.into_iter().collect::<Vec<_>>()
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
    pub fn join<I>(&self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut content = self.content.clone();

        for c in other {
            c.write_component(&mut content);
        }

        Self { content }
    }

    /// Clone and extend the item path.
    pub fn extended<C>(&self, part: C) -> Self
    where
        C: IntoComponent,
    {
        let mut content = self.content.clone();
        part.write_component(&mut content);
        Self { content }
    }

    /// Access the last component in the path.
    pub fn last(&self) -> Option<ComponentRef<'_>> {
        self.iter().next_back()
    }

    /// Implement an iterator.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            content: &self.content,
        }
    }

    /// Test if current item starts with another.
    pub fn starts_with(&self, other: &Self) -> bool {
        self.content.starts_with(&other.content)
    }

    /// Test if current is immediate super of `other`.
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
    pub fn ancestry(&self, other: &Self) -> (Self, Self) {
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
}

/// Format implementation for item.
///
/// An empty item is formatted as `{root}`, because it refers to the topmost
/// root module.
///
/// # Examples
///
/// ```rust
/// use runestick::{Item, ComponentRef::*};
///
/// assert_eq!("{root}", Item::new().to_string());
/// assert_eq!("hello::$0", Item::with_item(&[Str("hello"), Id(0)]).to_string());
/// ```
impl fmt::Display for Item {
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

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Item({})", self)
    }
}

impl<'a> IntoIterator for Item {
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

/// An item over the iterator.
///
/// Constructed using [Item::iter].
pub struct Iter<'a> {
    content: &'a [u8],
}

impl<'a> Iter<'a> {
    /// Check if the iterator is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Get the next component as a string.
    ///
    /// Will consume the next component in the iterator, but will only indicate
    /// if the next component was present, and was a [Component::Str].
    pub fn next_str(&mut self) -> Option<&'a str> {
        match self.next()? {
            ComponentRef::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Get the next back as a string component.
    ///
    /// Will consume the next component in the iterator, but will only indicate
    /// if the next component was present, and was a [Component::Str].
    pub fn next_back_str(&mut self) -> Option<&'a str> {
        match self.next_back()? {
            ComponentRef::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ComponentRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.content.is_empty() {
            return None;
        }

        let (head_tag, content) = self.content.split_at(TAG_BYTES);
        let (b, n) = read_tag(head_tag);

        let c = match b {
            CRATE => {
                let (s, content, tail_tag) = read_string(content, n);
                debug_assert_eq!(head_tag, tail_tag);
                self.content = content;
                return Some(ComponentRef::Crate(s));
            }
            STRING => {
                let (s, content, tail_tag) = read_string(content, n);
                debug_assert_eq!(head_tag, tail_tag);
                self.content = content;
                return Some(ComponentRef::Str(s));
            }
            ID => ComponentRef::Id(n),
            b => panic!("unsupported control byte {:?}", b),
        };

        self.content = content;
        return Some(c);

        fn read_string(content: &[u8], n: usize) -> (&str, &[u8], &[u8]) {
            let (buf, content) = content.split_at(n);

            // consume the head tag.
            let (tail_tag, content) = content.split_at(TAG_BYTES);

            // Safety: we control the construction of the item.
            let s = unsafe { std::str::from_utf8_unchecked(buf) };

            (s, content, tail_tag)
        }
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.content.is_empty() {
            return None;
        }

        let content = self.content;
        let (content, tail) = content.split_at(
            content
                .len()
                .checked_sub(TAG_BYTES)
                .expect("length underflow"),
        );
        let (b, n) = read_tag(tail);

        let c = match b {
            CRATE => {
                let (s, content) = read_string_back(content, n);
                self.content = content;
                return Some(ComponentRef::Crate(s));
            }
            STRING => {
                let (s, content) = read_string_back(content, n);
                self.content = content;
                return Some(ComponentRef::Str(s));
            }
            ID => ComponentRef::Id(n),
            b => panic!("unsupported control byte {:?}", b),
        };

        self.content = content;
        return Some(c);

        fn read_string_back(content: &[u8], n: usize) -> (&str, &[u8]) {
            let (content, buf) =
                content.split_at(content.len().checked_sub(n).expect("length underflow"));

            // consume the head tag.
            let (content, _) = content.split_at(
                content
                    .len()
                    .checked_sub(TAG_BYTES)
                    .expect("length underflow"),
            );

            // Safety: we control the construction of the item.
            let s = unsafe { std::str::from_utf8_unchecked(buf) };

            (s, content)
        }
    }
}

impl PartialEq<Item> for Iter<'_> {
    fn eq(&self, other: &Item) -> bool {
        self.content == other.content
    }
}

impl PartialEq<&Item> for Iter<'_> {
    fn eq(&self, other: &&Item) -> bool {
        self.content == other.content
    }
}

impl PartialEq<Iter<'_>> for Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        self.content == other.content
    }
}

impl PartialEq<Iter<'_>> for &Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        self.content == other.content
    }
}

/// The component of an item.
///
/// All indexes refer to sibling indexes. So two sibling id components could
/// have the indexes `1` and `2` respectively.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Component {
    /// A crate component.
    Crate(Box<str>),
    /// A regular string component.
    Str(Box<str>),
    /// A nested anonymous part with an identifier.
    Id(usize),
}

impl Component {
    /// Get the identifier of the component.
    pub fn id(&self) -> Option<usize> {
        match self {
            Self::Id(n) => Some(*n),
            _ => None,
        }
    }

    /// Convert into component reference.
    pub fn as_component_ref(&self) -> ComponentRef<'_> {
        match self {
            Self::Crate(s) => ComponentRef::Crate(&*s),
            Self::Str(s) => ComponentRef::Str(&*s),
            Self::Id(n) => ComponentRef::Id(*n),
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crate(s) => write!(fmt, "::{}", s),
            Self::Str(s) => write!(fmt, "{}", s),
            Self::Id(n) => write!(fmt, "${}", n),
        }
    }
}

/// A reference to a component of an item.
///
/// All indexes refer to sibling indexes. So two sibling id components could
/// have the indexes `1` and `2` respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComponentRef<'a> {
    /// A crate string component.
    Crate(&'a str),
    /// A regular string component.
    Str(&'a str),
    /// A nested anonymous part with an identifier.
    Id(usize),
}

impl ComponentRef<'_> {
    /// Get the identifier of the component if it is an identifier component.
    pub fn id(self) -> Option<usize> {
        match self {
            Self::Id(n) => Some(n),
            _ => None,
        }
    }

    /// Convert into an owned component.
    pub fn into_component(self) -> Component {
        match self {
            Self::Crate(s) => Component::Crate(s.into()),
            Self::Str(s) => Component::Str(s.into()),
            Self::Id(n) => Component::Id(n),
        }
    }

    /// Write the current component to the given vector.
    pub fn write_component(self, output: &mut Vec<u8>) {
        match self {
            ComponentRef::Crate(s) => {
                write_crate(s, output);
            }
            ComponentRef::Str(s) => {
                write_str(s, output);
            }
            ComponentRef::Id(c) => {
                write_tag(output, ID, c);
            }
        }
    }

    /// Hash the current component to the given hasher.
    pub fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        match self {
            ComponentRef::Crate(s) => {
                CRATE.hash(hasher);
                s.hash(hasher);
            }
            ComponentRef::Str(s) => {
                STRING.hash(hasher);
                s.hash(hasher);
            }
            ComponentRef::Id(c) => {
                ID.hash(hasher);
                c.hash(hasher);
            }
        }
    }
}

impl fmt::Display for ComponentRef<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crate(s) => write!(fmt, "::{}", s),
            Self::Str(s) => write!(fmt, "{}", s),
            Self::Id(n) => write!(fmt, "${}", n),
        }
    }
}

/// Trait for encoding the current type into a component.
pub trait IntoComponent: Sized {
    /// Convert into a component directly.
    fn as_component_ref(&self) -> ComponentRef<'_>;

    /// Convert into component.
    fn into_component(self) -> Component {
        ComponentRef::into_component(self.as_component_ref())
    }

    /// Write a component directly to a buffer.
    fn write_component(self, output: &mut Vec<u8>) {
        ComponentRef::write_component(self.as_component_ref(), output)
    }

    /// Hash the current component.
    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        ComponentRef::hash_component(self.as_component_ref(), hasher)
    }
}

impl IntoComponent for ComponentRef<'_> {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        *self
    }

    fn into_component(self) -> Component {
        ComponentRef::into_component(self)
    }
}

impl IntoComponent for &ComponentRef<'_> {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        **self
    }

    fn into_component(self) -> Component {
        ComponentRef::into_component(*self)
    }
}

impl IntoComponent for Component {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(self)
    }

    fn into_component(self) -> Component {
        self
    }
}

impl IntoComponent for &Component {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(*self)
    }

    fn into_component(self) -> Component {
        self.clone()
    }
}

macro_rules! impl_into_component_for_str {
    ($ty:ty, $slf:ident, $into:expr) => {
        impl IntoComponent for $ty {
            fn as_component_ref(&self) -> ComponentRef<'_> {
                ComponentRef::Str(self.as_ref())
            }

            fn into_component($slf) -> Component {
                Component::Str($into)
            }

            fn write_component(self, output: &mut Vec<u8>) {
                write_str(self.as_ref(), output)
            }

            fn hash_component<H>(self, hasher: &mut H)
            where
                H: hash::Hasher,
            {
                hash_str(self.as_ref(), hasher);
            }
        }
    }
}

impl_into_component_for_str!(&str, self, self.into());
impl_into_component_for_str!(&&str, self, (*self).into());
impl_into_component_for_str!(RawStr, self, (*self).into());
impl_into_component_for_str!(&RawStr, self, (**self).into());
impl_into_component_for_str!(String, self, self.into());
impl_into_component_for_str!(&String, self, self.clone().into());
impl_into_component_for_str!(std::borrow::Cow<'_, str>, self, self.as_ref().into());

/// Read a single byte.
///
/// # Panics
///
/// Panics if the byte is not available.
fn read_tag(content: &[u8]) -> (u8, usize) {
    let n = NativeEndian::read_u16(content);
    let n = usize::try_from(n).unwrap();
    ((n & TYPE_MASK) as u8, n >> TYPE_BITS)
}

/// Helper function to write an identifier.
///
/// # Panics
///
/// Panics if the provided size cannot fit withing an identifier.
fn write_tag(output: &mut Vec<u8>, tag: u8, n: usize) {
    debug_assert!(tag as usize <= TYPE_MASK);
    assert!(
        n < MAX_DATA,
        "item data overflow, index or string size larger than MAX_DATA"
    );
    let n = u16::try_from(n << TYPE_BITS | tag as usize).unwrap();
    output.write_u16::<NativeEndian>(n).unwrap();
}

/// Internal function to write only the crate of a component.
fn write_crate(s: &str, output: &mut Vec<u8>) {
    write_tag(output, CRATE, s.len());
    output.extend(s.as_bytes());
    write_tag(output, CRATE, s.len());
}

/// Internal function to write only the string of a component.
fn write_str(s: &str, output: &mut Vec<u8>) {
    write_tag(output, STRING, s.len());
    output.extend(s.as_bytes());
    write_tag(output, STRING, s.len());
}

/// Internal function to hash the given string.
fn hash_str<H>(string: &str, hasher: &mut H)
where
    H: hash::Hasher,
{
    STRING.hash(hasher);
    string.hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::{Component, ComponentRef, IntoComponent as _, Item};

    #[test]
    fn test_pop() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Id(1));
        item.push(ComponentRef::Id(2));
        item.push("middle");
        item.push(ComponentRef::Id(3));
        item.push("end");

        assert_eq!(item.pop(), Some("end".into_component()));
        assert_eq!(item.pop(), Some(Component::Id(3)));
        assert_eq!(item.pop(), Some("middle".into_component()));
        assert_eq!(item.pop(), Some(Component::Id(2)));
        assert_eq!(item.pop(), Some(Component::Id(1)));
        assert_eq!(item.pop(), Some("start".into_component()));
        assert_eq!(item.pop(), None);

        assert!(item.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Id(1));
        item.push(ComponentRef::Id(2));
        item.push("middle");
        item.push(ComponentRef::Id(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next(), Some("start".as_component_ref()));
        assert_eq!(it.next(), Some(ComponentRef::Id(1)));
        assert_eq!(it.next(), Some(ComponentRef::Id(2)));
        assert_eq!(it.next(), Some("middle".as_component_ref()));
        assert_eq!(it.next(), Some(ComponentRef::Id(3)));
        assert_eq!(it.next(), Some("end".as_component_ref()));
        assert_eq!(it.next(), None);

        assert!(!item.is_empty());
    }

    #[test]
    fn test_next_back_str() {
        let mut item = Item::new();

        item.push(ComponentRef::Crate("std"));
        item.push("start");
        item.push(ComponentRef::Id(1));
        item.push(ComponentRef::Id(2));
        item.push("middle");
        item.push(ComponentRef::Id(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next_back_str(), Some("end"));
        assert_eq!(it.next_back(), Some(ComponentRef::Id(3)));
        assert_eq!(it.next_back_str(), Some("middle"));
        assert_eq!(it.next_back(), Some(ComponentRef::Id(2)));
        assert_eq!(it.next_back(), Some(ComponentRef::Id(1)));
        assert_eq!(it.next_back_str(), Some("start"));
        assert_eq!(it.next_back(), Some(ComponentRef::Crate("std")));
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn alternate() {
        let mut item = Item::new();

        item.push(ComponentRef::Crate("std"));
        item.push("start");
        item.push(ComponentRef::Id(1));
        item.push(ComponentRef::Id(2));
        item.push("middle");
        item.push(ComponentRef::Id(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next(), Some(ComponentRef::Crate("std")));
        assert_eq!(it.next_str(), Some("start"));
        assert_eq!(it.next_back_str(), Some("end"));
        assert_eq!(it.next(), Some(ComponentRef::Id(1)));
        assert_eq!(it.next(), Some(ComponentRef::Id(2)));
        assert_eq!(it.next_back(), Some(ComponentRef::Id(3)));
        assert_eq!(it.next_str(), Some("middle"));
        assert_eq!(it.next_back(), None);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn store_max_data() {
        let mut item = Item::new();
        item.push(ComponentRef::Id(super::MAX_DATA - 1));
        assert_eq!(item.last(), Some(ComponentRef::Id(super::MAX_DATA - 1)));
    }

    #[test]
    fn store_max_string() {
        let mut item = Item::new();
        let s = "x".repeat(super::MAX_DATA - 1);
        item.push(ComponentRef::Str(&s));
        assert_eq!(item.last(), Some(ComponentRef::Str(&s)));
    }

    #[test]
    #[should_panic(expected = "item data overflow, index or string size larger than MAX_DATA")]
    fn store_max_data_overflow() {
        let mut item = Item::new();
        item.push(ComponentRef::Id(super::MAX_DATA));
        assert_eq!(item.last(), Some(ComponentRef::Id(super::MAX_DATA)));
    }

    #[test]
    #[should_panic(expected = "item data overflow, index or string size larger than MAX_DATA")]
    fn store_max_string_overflow() {
        let mut item = Item::new();
        let s = "x".repeat(super::MAX_DATA);
        item.push(ComponentRef::Str(&s));
    }

    #[test]
    fn test_is_super_of() {
        assert!(Item::new().is_super_of(&Item::new(), 1));
        assert!(!Item::with_item(&["a"]).is_super_of(&Item::new(), 1));

        assert!(!Item::with_item(&["a", "b"]).is_super_of(&Item::with_item(&["a"]), 1));
        assert!(Item::with_item(&["a", "b"]).is_super_of(&Item::with_item(&["a", "b"]), 1));
        assert!(!Item::with_item(&["a"]).is_super_of(&Item::with_item(&["a", "b", "c"]), 1));
    }

    #[test]
    fn test_ancestry() {
        assert_eq!(
            (Item::new(), Item::new()),
            Item::new().ancestry(&Item::new())
        );

        assert_eq!(
            (Item::new(), Item::with_item(&["a"])),
            Item::new().ancestry(&Item::with_item(&["a"]))
        );

        assert_eq!(
            (Item::new(), Item::with_item(&["a", "b"])),
            Item::new().ancestry(&Item::with_item(&["a", "b"]))
        );

        assert_eq!(
            (Item::with_item(&["a"]), Item::with_item(&["b"])),
            Item::with_item(&["a", "c"]).ancestry(&Item::with_item(&["a", "b"]))
        );

        assert_eq!(
            (Item::with_item(&["a", "b"]), Item::with_item(&["d", "e"])),
            Item::with_item(&["a", "b", "c"]).ancestry(&Item::with_item(&["a", "b", "d", "e"]))
        );
    }
}
