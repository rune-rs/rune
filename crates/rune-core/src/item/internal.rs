use core::hash::{self, Hash};
use core::str;

use crate::alloc::alloc::Allocator;
use crate::alloc::{self, Vec};

// Types available.
pub(super) const CRATE: Tag = Tag(0b00);
pub(super) const STRING: Tag = Tag(0b01);
pub(super) const ID: Tag = Tag(0b10);

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
    let &[a, b] = content else {
        panic!("expected two bytes");
    };

    let n = u16::from_ne_bytes([a, b]);
    let n = usize::from(n);
    (Tag((n & TYPE_MASK) as u8), n >> TYPE_BITS)
}

/// Test if the given bytes are the encoding of an item.
///
/// An item is a sequence of components. An identifier is a single tag, while a
/// crate or a string is a tag, followed by that many bytes of UTF-8, followed
/// by the same tag again so that it can be read from the back.
#[cfg(any(feature = "musli", test))]
pub(super) fn validate(mut content: &[u8]) -> bool {
    while !content.is_empty() {
        let Some((head_tag, rest)) = content.split_at_checked(TAG_BYTES) else {
            return false;
        };

        content = match read_tag(head_tag) {
            (ID, _) => rest,
            (CRATE | STRING, n) => {
                let Some((string, rest)) = rest.split_at_checked(n) else {
                    return false;
                };

                let Some((tail_tag, rest)) = rest.split_at_checked(TAG_BYTES) else {
                    return false;
                };

                if head_tag != tail_tag || str::from_utf8(string).is_err() {
                    return false;
                }

                rest
            }
            _ => return false,
        };
    }

    true
}

/// Helper function to write an identifier.
///
/// A component longer than [`MAX_DATA`] does not fit in the tag, which is an
/// error rather than an assertion: the size is a name out of a source file, so
/// asserting on it turns any script carrying a long enough identifier into an
/// abort - in a debug build only, which is where a host runs its tests.
pub(super) fn write_tag<A>(output: &mut Vec<u8, A>, Tag(tag): Tag, n: usize) -> alloc::Result<()>
where
    A: Allocator,
{
    let tag = usize::from(tag);

    debug_assert!(tag <= TYPE_MASK);

    if n >= MAX_DATA {
        return Err(alloc::Error::CapacityOverflow);
    }

    let n = u16::try_from((n << TYPE_BITS) | tag).expect("tag out of bounds");
    let buf = n.to_ne_bytes();
    output.try_extend_from_slice(&buf[..])?;
    Ok(())
}

/// Internal function to write only the crate of a component.
pub(super) fn write_crate<A>(s: &str, output: &mut Vec<u8, A>) -> alloc::Result<()>
where
    A: Allocator,
{
    write_tag(output, CRATE, s.len())?;
    output.try_extend_from_slice(s.as_bytes())?;
    write_tag(output, CRATE, s.len())?;
    Ok(())
}

/// Internal function to write only the string of a component.
pub(super) fn write_str<A>(s: &str, output: &mut Vec<u8, A>) -> alloc::Result<()>
where
    A: Allocator,
{
    write_tag(output, STRING, s.len())?;
    output.try_extend_from_slice(s.as_bytes())?;
    write_tag(output, STRING, s.len())?;
    Ok(())
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
