use crate::RawStr;
use byteorder::{ByteOrder as _, LittleEndian, ReadBytesExt as _, WriteBytesExt as _};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom as _;
use std::fmt;
use std::hash;
use std::hash::Hash as _;
use std::io::Cursor;
use std::io::Read as _;

const STRING: u8 = 0;
const BLOCK: u8 = 1;
const CLOSURE: u8 = 2;
const ASYNC_BLOCK: u8 = 3;

/// The name of an item.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
///
/// # Panics
///
/// The max length of an item is 2**16 = 65536. Attempting to create an item
/// larger than that will panic.
///
/// # Component encoding
///
/// A component is encoded as:
/// * A single byte prefix identifying the type of the component.
/// * The payload of the component, specific to its type.
/// * The offset to the start of the last component.
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
    pub fn of<I>(iter: I) -> Self
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

    /// Check if the item is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
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

    /// Construct a new vector from the current item.
    pub fn as_vec(&self) -> Vec<Component> {
        self.iter()
            .map(ComponentRef::into_component)
            .collect::<Vec<_>>()
    }

    /// Convert into a vector from the current item.
    pub fn into_vec(self) -> Vec<Component> {
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
        self.iter().next_back_ref()
    }

    /// Implement an iterator.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            content: &self.content,
        }
    }
}

/// Format implementation for item.
///
/// An empty item is formatted as `{empty}`.
///
/// # Examples
///
/// ```rust
/// use runestick::{Item, Component::*};
///
/// assert_eq!("{empty}", Item::new().to_string());
/// assert_eq!("hello::$block0", Item::of(&[String("hello".into()), Block(0)]).to_string());
/// ```
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.iter();

        if let Some(last) = it.next_back() {
            for p in it {
                write!(f, "{}::", p)?;
            }

            write!(f, "{}", last)
        } else {
            write!(f, "{{empty}}")
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
    /// if the next component was present, and was a [Component::String].
    pub fn next_str(&mut self) -> Option<&'a str> {
        if self.content.is_empty() {
            return None;
        }

        let mut cursor = Cursor::new(&self.content[..]);
        let c = ComponentRef::decode_str(&mut cursor);
        let start = usize::try_from(cursor.position()).unwrap();
        self.content = &self.content[start..];
        c
    }

    /// Get the next back as a string component.
    ///
    /// Will consume the next component in the iterator, but will only indicate
    /// if the next component was present, and was a [Component::String].
    pub fn next_back_str(&mut self) -> Option<&'a str> {
        if self.content.is_empty() {
            return None;
        }

        let (head, tail) = split_tail(&self.content);
        self.content = head;
        ComponentRef::decode_str(&mut Cursor::new(tail))
    }

    /// Get the next back as a component reference.
    ///
    /// Will consume the next component in the iterator.
    pub fn next_back_ref(&mut self) -> Option<ComponentRef<'a>> {
        if self.content.is_empty() {
            return None;
        }

        let (head, tail) = split_tail(&self.content);
        self.content = head;
        Some(ComponentRef::decode(&mut Cursor::new(tail)))
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ComponentRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.content.is_empty() {
            return None;
        }

        let mut cursor = Cursor::new(&self.content[..]);
        let c = ComponentRef::decode(&mut cursor);
        let start = usize::try_from(cursor.position()).unwrap();
        self.content = &self.content[start..];
        Some(c)
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.content.is_empty() {
            return None;
        }

        let (head, tail) = split_tail(&self.content);
        self.content = head;
        let c = ComponentRef::decode(&mut Cursor::new(tail));
        Some(c)
    }
}

/// The component of an item.
///
/// All indexes refere to sibling indexes. So two sibling blocks could have the
/// indexes `1` and `2` respectively.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Component {
    /// A regular string component.
    String(Box<str>),
    /// A nested block with an index.
    Block(usize),
    /// A closure component.
    Closure(usize),
    /// An async block, like `async {  }`.
    AsyncBlock(usize),
}

impl Component {
    /// Get the identifier of the component.
    pub fn id(&self) -> Option<usize> {
        match self {
            Self::String(..) => None,
            Self::Block(n) => Some(*n),
            Self::Closure(n) => Some(*n),
            Self::AsyncBlock(n) => Some(*n),
        }
    }

    /// Convert into component reference.
    pub fn as_component_ref(&self) -> ComponentRef<'_> {
        match self {
            Self::String(s) => ComponentRef::String(&*s),
            Self::Block(n) => ComponentRef::Block(*n),
            Self::Closure(n) => ComponentRef::Closure(*n),
            Self::AsyncBlock(n) => ComponentRef::AsyncBlock(*n),
        }
    }

    /// Internal function to write only the string of a component.
    fn write_str(output: &mut Vec<u8>, s: &str) {
        output.push(STRING);
        let len = u16::try_from(s.len()).unwrap();
        output.write_u16::<LittleEndian>(len).unwrap();
        output.extend(s.as_bytes());
    }
}

impl fmt::Display for Component {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(fmt, "{}", s),
            Self::Block(n) => write!(fmt, "$block{}", n),
            Self::Closure(n) => write!(fmt, "$closure{}", n),
            Self::AsyncBlock(n) => write!(fmt, "$async{}", n),
        }
    }
}

/// A reference to a component of an item.
///
/// All indexes refere to sibling indexes. So two sibling blocks could have the
/// indexes `1` and `2` respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComponentRef<'a> {
    /// A regular string component.
    String(&'a str),
    /// A nested block with an index.
    Block(usize),
    /// A closure component.
    Closure(usize),
    /// An async block, like `async {  }`.
    AsyncBlock(usize),
}

impl ComponentRef<'_> {
    /// Get the identifier of the component.
    pub fn id(self) -> Option<usize> {
        match self {
            Self::String(..) => None,
            Self::Block(n) => Some(n),
            Self::Closure(n) => Some(n),
            Self::AsyncBlock(n) => Some(n),
        }
    }

    /// Convert into an owned component.
    pub fn into_component(self) -> Component {
        match self {
            Self::String(s) => Component::String(s.into()),
            Self::Block(n) => Component::Block(n),
            Self::Closure(n) => Component::Closure(n),
            Self::AsyncBlock(n) => Component::AsyncBlock(n),
        }
    }

    /// Internal function to decode a borrowed string component without cloning
    /// it.
    fn decode_str<'a>(content: &mut Cursor<&'a [u8]>) -> Option<&'a str> {
        match Self::decode(content) {
            ComponentRef::String(s) => Some(s),
            _ => None,
        }
    }

    /// Internal function to decode a borrowed component without cloning it.
    fn decode<'a>(content: &mut Cursor<&'a [u8]>) -> ComponentRef<'a> {
        let c = match read_u8(content) {
            STRING => {
                let len = read_usize(content);
                let bytes = read_bytes(content, len);

                // Safety: all code paths which construct a string component
                // are safe input paths which ensure that the input is a string.
                ComponentRef::String(unsafe { std::str::from_utf8_unchecked(bytes) })
            }
            BLOCK => ComponentRef::Block(read_usize(content)),
            CLOSURE => ComponentRef::Closure(read_usize(content)),
            ASYNC_BLOCK => ComponentRef::AsyncBlock(read_usize(content)),
            b => panic!("unexpected control byte `{:?}`", b),
        };

        // read the suffix offset used for reading backwards.
        let _ = read_usize(content);
        c
    }
}

impl fmt::Display for ComponentRef<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(fmt, "{}", s),
            Self::Block(n) => write!(fmt, "$block{}", n),
            Self::Closure(n) => write!(fmt, "$closure{}", n),
            Self::AsyncBlock(n) => write!(fmt, "$async{}", n),
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
        IntoComponent::write_component(self.as_component_ref(), output)
    }

    /// Hash the current component.
    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        IntoComponent::hash_component(self.as_component_ref(), hasher)
    }
}

impl IntoComponent for ComponentRef<'_> {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        *self
    }

    fn into_component(self) -> Component {
        ComponentRef::into_component(self)
    }

    fn write_component(self, output: &mut Vec<u8>) {
        write_with_suffix_len(output, |output| match self {
            ComponentRef::String(s) => {
                Component::write_str(output, s);
            }
            ComponentRef::Block(c) => {
                output.push(BLOCK);
                write_usize(output, c);
            }
            ComponentRef::Closure(c) => {
                output.push(CLOSURE);
                write_usize(output, c);
            }
            ComponentRef::AsyncBlock(c) => {
                output.push(ASYNC_BLOCK);
                write_usize(output, c);
            }
        })
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        match self {
            ComponentRef::String(s) => {
                STRING.hash(hasher);
                s.hash(hasher);
            }
            ComponentRef::Block(c) => {
                BLOCK.hash(hasher);
                c.hash(hasher);
            }
            ComponentRef::Closure(c) => {
                CLOSURE.hash(hasher);
                c.hash(hasher);
            }
            ComponentRef::AsyncBlock(c) => {
                ASYNC_BLOCK.hash(hasher);
                c.hash(hasher);
            }
        }
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

impl IntoComponent for &str {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(*self)
    }

    fn into_component(self) -> Component {
        Component::String(self.to_owned().into_boxed_str())
    }

    /// Encode the given string onto the buffer.
    fn write_component(self, output: &mut Vec<u8>) {
        write_with_suffix_len(output, |output| Component::write_str(output, self))
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        STRING.hash(hasher);
        self.hash(hasher);
    }
}

impl IntoComponent for RawStr {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(&**self)
    }

    fn into_component(self) -> Component {
        Component::String((*self).to_owned().into_boxed_str())
    }

    fn write_component(self, output: &mut Vec<u8>) {
        <&str>::write_component(&*self, output)
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        <&str>::hash_component(&*self, hasher);
    }
}

impl IntoComponent for &RawStr {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(&***self)
    }

    fn into_component(self) -> Component {
        Component::String((**self).to_owned().into_boxed_str())
    }

    fn write_component(self, output: &mut Vec<u8>) {
        <&str>::write_component(&**self, output)
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        <&str>::hash_component(&**self, hasher);
    }
}

impl IntoComponent for &&str {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(**self)
    }

    fn into_component(self) -> Component {
        Component::String((*self).to_owned().into_boxed_str())
    }

    fn write_component(self, output: &mut Vec<u8>) {
        <&str>::write_component(*self, output)
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        <&str>::hash_component(*self, hasher);
    }
}

impl IntoComponent for String {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(&*self)
    }

    fn into_component(self) -> Component {
        Component::String(self.into_boxed_str())
    }

    fn write_component(self, output: &mut Vec<u8>) {
        <&str>::write_component(self.as_str(), output)
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        <&str>::hash_component(self.as_str(), hasher)
    }
}

impl IntoComponent for &String {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        ComponentRef::String(&**self)
    }

    fn into_component(self) -> Component {
        Component::String(self.clone().into_boxed_str())
    }

    fn write_component(self, output: &mut Vec<u8>) {
        <&str>::write_component(self.as_str(), output)
    }

    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        <&str>::hash_component(self.as_str(), hasher);
    }
}

/// Split the tail end of the content buffer.
fn split_tail(content: &[u8]) -> (&[u8], &[u8]) {
    let start = content.len().checked_sub(2).unwrap();
    let len = LittleEndian::read_u16(&content[start..]);
    let len = usize::try_from(len).unwrap();
    let start = start.checked_sub(len).unwrap();
    let start = usize::try_from(start).unwrap();
    content.split_at(start)
}

/// Read a single byte from the cursor.
fn read_u8(cursor: &mut Cursor<&[u8]>) -> u8 {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf).unwrap();
    buf[0]
}

/// Read a usize out of the cursor.
///
/// Internally we encode usize's as LE u32's.
///
/// # Panics
///
/// panics if the cursor doesn't contain enough data to decode.
fn read_usize(cursor: &mut Cursor<&[u8]>) -> usize {
    let c = cursor.read_u16::<LittleEndian>().unwrap();
    usize::try_from(c).unwrap()
}

/// Helper function to write a usize.
///
/// # Panics
///
/// Panics if the provided value cannot fit in a u16.
fn write_usize(output: &mut Vec<u8>, value: usize) {
    let value = u16::try_from(value).unwrap();
    output.write_u16::<LittleEndian>(value).unwrap();
}

/// Read the given number of bytes from the cursor without copying them.
///
/// # Panics
///
/// Panics if the provided number of bytes are not available in the cursor.
fn read_bytes<'a>(cursor: &mut Cursor<&'a [u8]>, len: usize) -> &'a [u8] {
    let pos = usize::try_from(cursor.position()).unwrap();
    let end = pos.checked_add(len).unwrap();
    let bytes = &(*cursor.get_ref())[pos..end];
    let end = u64::try_from(end).unwrap();
    cursor.set_position(end);
    bytes
}

/// Perform the given write operation, while adding the suffix length to the end
/// of it. This is used when reading the items backwards (e.g.
/// [Iter::next_back_ref]).
fn write_with_suffix_len(output: &mut Vec<u8>, cb: impl FnOnce(&mut Vec<u8>)) {
    let offset = output.len();
    cb(output);
    write_usize(output, output.len() - offset);
}

#[cfg(test)]
mod tests {
    use super::{Component, ComponentRef, IntoComponent as _, Item};

    #[test]
    fn test_pop() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Block(1));
        item.push(ComponentRef::Closure(2));
        item.push("middle");
        item.push(ComponentRef::AsyncBlock(3));
        item.push("end");

        assert_eq!(item.pop(), Some("end".into_component()));
        assert_eq!(item.pop(), Some(Component::AsyncBlock(3)));
        assert_eq!(item.pop(), Some("middle".into_component()));
        assert_eq!(item.pop(), Some(Component::Closure(2)));
        assert_eq!(item.pop(), Some(Component::Block(1)));
        assert_eq!(item.pop(), Some("start".into_component()));
        assert_eq!(item.pop(), None);

        assert!(item.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Block(1));
        item.push(ComponentRef::Closure(2));
        item.push("middle");
        item.push(ComponentRef::AsyncBlock(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next(), Some("start".as_component_ref()));
        assert_eq!(it.next(), Some(ComponentRef::Block(1)));
        assert_eq!(it.next(), Some(ComponentRef::Closure(2)));
        assert_eq!(it.next(), Some("middle".as_component_ref()));
        assert_eq!(it.next(), Some(ComponentRef::AsyncBlock(3)));
        assert_eq!(it.next(), Some("end".as_component_ref()));
        assert_eq!(it.next(), None);

        assert!(!item.is_empty());
    }

    #[test]
    fn test_next_back_str() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Block(1));
        item.push(ComponentRef::Closure(2));
        item.push("middle");
        item.push(ComponentRef::AsyncBlock(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next_back_str(), Some("end"));
        assert_eq!(it.next_back(), Some(ComponentRef::AsyncBlock(3)));
        assert_eq!(it.next_back_str(), Some("middle"));
        assert_eq!(it.next_back(), Some(ComponentRef::Closure(2)));
        assert_eq!(it.next_back(), Some(ComponentRef::Block(1)));
        assert_eq!(it.next_back_str(), Some("start"));
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn alternate() {
        let mut item = Item::new();

        item.push("start");
        item.push(ComponentRef::Block(1));
        item.push(ComponentRef::Closure(2));
        item.push("middle");
        item.push(ComponentRef::AsyncBlock(3));
        item.push("end");

        let mut it = item.iter();

        assert_eq!(it.next_str(), Some("start"));
        assert_eq!(it.next_back_str(), Some("end"));
        assert_eq!(it.next(), Some(ComponentRef::Block(1)));
        assert_eq!(it.next(), Some(ComponentRef::Closure(2)));
        assert_eq!(it.next_back(), Some(ComponentRef::AsyncBlock(3)));
        assert_eq!(it.next_str(), Some("middle"));
        assert_eq!(it.next_back(), None);
        assert_eq!(it.next(), None);
    }
}
