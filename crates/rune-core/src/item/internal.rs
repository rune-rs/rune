use core::hash::{self, Hash};
use core::str;

use byteorder::{ByteOrder, NativeEndian};
use smallvec::SmallVec;

// Types available.
pub(super) const CRATE: Tag = Tag(0b00);
pub(super) const STRING: Tag = Tag(0b01);
pub(super) const ID: Tag = Tag(0b10);

/// Inline size.
pub(super) const INLINE: usize = 32;
/// How many bits the type of a tag takes up.
pub(super) const TYPE_BITS: usize = 2;
/// Mask of the type of a tag.
pub(super) const TYPE_MASK: usize = (0b1 << TYPE_BITS) - 1;
/// Total tag size in bytes.
pub(super) const TAG_BYTES: usize = 2;
/// Max size of data stored.
pub(super) const MAX_DATA: usize = 0b1 << (TAG_BYTES * 8 - TYPE_BITS);

#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(super) struct Tag(pub(super) u8);

/// Read a single byte.
///
/// # Panics
///
/// Panics if the byte is not available.
pub(super) fn read_tag(content: &[u8]) -> (Tag, usize) {
    let n = NativeEndian::read_u16(content);
    let n = usize::try_from(n).unwrap();
    (Tag((n & TYPE_MASK) as u8), n >> TYPE_BITS)
}

/// Helper function to write an identifier.
///
/// # Panics
///
/// Panics if the provided size cannot fit withing an identifier.
pub(super) fn write_tag(output: &mut SmallVec<[u8; INLINE]>, Tag(tag): Tag, n: usize) {
    let tag = usize::try_from(tag).expect("tag out of bounds");
    debug_assert!(tag <= TYPE_MASK);
    assert!(
        n < MAX_DATA,
        "item data overflow, index or string size larger than MAX_DATA"
    );
    let n = u16::try_from(n << TYPE_BITS | tag).expect("tag out of bounds");
    let mut buf = [0, 0];
    NativeEndian::write_u16(&mut buf[..], n);
    output.extend_from_slice(&buf[..]);
}

/// Internal function to write only the crate of a component.
pub(super) fn write_crate(s: &str, output: &mut SmallVec<[u8; INLINE]>) {
    write_tag(output, CRATE, s.len());
    output.extend_from_slice(s.as_bytes());
    write_tag(output, CRATE, s.len());
}

/// Internal function to write only the string of a component.
pub(super) fn write_str(s: &str, output: &mut SmallVec<[u8; INLINE]>) {
    write_tag(output, STRING, s.len());
    output.extend_from_slice(s.as_bytes());
    write_tag(output, STRING, s.len());
}

/// Internal function to hash the given string.
pub(super) fn hash_str<H>(string: &str, hasher: &mut H)
where
    H: hash::Hasher,
{
    STRING.hash(hasher);
    string.hash(hasher);
}

pub(super) fn read_string(content: &[u8], n: usize) -> (&str, &[u8], &[u8]) {
    let (buf, content) = content.split_at(n);

    // consume the head tag.
    let (tail_tag, content) = content.split_at(TAG_BYTES);

    // Safety: we control the construction of the item.
    let s = unsafe { str::from_utf8_unchecked(buf) };

    (s, content, tail_tag)
}
