use crate::compile::item::internal;
use crate::compile::item::{ComponentRef, Item, ItemRef};

/// An item over the iterator.
///
/// Constructed using [Item::iter].
pub struct Iter<'a> {
    content: &'a [u8],
}

impl<'a> Iter<'a> {
    /// Constructor for an iterator.
    pub(super) fn new(content: &'a [u8]) -> Self {
        Self { content }
    }

    /// The length of the content being held by the iterator.
    pub(super) fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the iterator is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Coerce the iterator into an item.
    pub fn as_item(&self) -> &ItemRef {
        // SAFETY: Iterator ensures that content is valid.
        unsafe { ItemRef::new(self.content.as_ref()) }
    }

    /// Coerce the iterator into an item with the lifetime of the iterator.
    pub fn into_item(&self) -> &'a ItemRef {
        // SAFETY: Iterator ensures that content is valid.
        unsafe { ItemRef::new(self.content.as_ref()) }
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

        let (head_tag, content) = self.content.split_at(internal::TAG_BYTES);
        let (b, n) = internal::read_tag(head_tag);

        let c = match b {
            internal::CRATE => {
                let (s, content, tail_tag) = internal::read_string(content, n);
                debug_assert_eq!(head_tag, tail_tag);
                self.content = content;
                return Some(ComponentRef::Crate(s));
            }
            internal::STRING => {
                let (s, content, tail_tag) = internal::read_string(content, n);
                debug_assert_eq!(head_tag, tail_tag);
                self.content = content;
                return Some(ComponentRef::Str(s));
            }
            internal::ID => ComponentRef::Id(n),
            internal::Tag(b) => panic!("unsupported control byte {:?}", b),
        };

        self.content = content;
        Some(c)
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
                .checked_sub(internal::TAG_BYTES)
                .expect("length underflow"),
        );
        let (b, n) = internal::read_tag(tail);

        let c = match b {
            internal::CRATE => {
                let (s, content) = read_string_back(content, n);
                self.content = content;
                return Some(ComponentRef::Crate(s));
            }
            internal::STRING => {
                let (s, content) = read_string_back(content, n);
                self.content = content;
                return Some(ComponentRef::Str(s));
            }
            internal::ID => ComponentRef::Id(n),
            internal::Tag(b) => panic!("unsupported control byte {:?}", b),
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
                    .checked_sub(internal::TAG_BYTES)
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
        self.content == other.content.as_ref()
    }
}

impl PartialEq<&Item> for Iter<'_> {
    fn eq(&self, other: &&Item) -> bool {
        self.content == other.content.as_ref()
    }
}

impl PartialEq<Iter<'_>> for Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        self.content.as_ref() == other.content
    }
}

impl PartialEq<Iter<'_>> for &Item {
    fn eq(&self, other: &Iter<'_>) -> bool {
        self.content.as_ref() == other.content
    }
}

impl PartialEq<ItemRef> for Iter<'_> {
    fn eq(&self, other: &ItemRef) -> bool {
        self.content == &other.content
    }
}

impl PartialEq<&ItemRef> for Iter<'_> {
    fn eq(&self, other: &&ItemRef) -> bool {
        self.content == &other.content
    }
}

impl PartialEq<Iter<'_>> for ItemRef {
    fn eq(&self, other: &Iter<'_>) -> bool {
        &self.content == other.content
    }
}

impl PartialEq<Iter<'_>> for &ItemRef {
    fn eq(&self, other: &Iter<'_>) -> bool {
        &self.content == other.content
    }
}
