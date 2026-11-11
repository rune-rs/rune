use crate::item::internal::{self, MAX_DATA};
use crate::item::{ComponentRef, ItemBuf};
use rune_alloc as alloc;

#[test]
fn test_pop() -> alloc::Result<()> {
    let mut item = ItemBuf::new();

    item.push("start")?;
    item.push(ComponentRef::Id(1))?;
    item.push(ComponentRef::Id(2))?;
    item.push("middle")?;
    item.push(ComponentRef::Id(3))?;
    item.push("end")?;

    assert!(item.pop());
    assert!(item.pop());
    assert!(item.pop());
    assert!(item.pop());
    assert!(item.pop());
    assert!(item.pop());
    assert!(!item.pop());

    assert!(item.is_empty());
    Ok(())
}

#[test]
fn test_back_iter() -> alloc::Result<()> {
    let mut item = ItemBuf::new();

    item.push("start")?;
    item.push(ComponentRef::Id(1))?;
    item.push(ComponentRef::Id(2))?;
    item.push("middle")?;
    item.push(ComponentRef::Id(3))?;
    item.push("end")?;

    let mut it = item.iter();

    assert_eq!(it.next_back(), Some(ComponentRef::Str("end")));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(3)));
    assert_eq!(it.next_back(), Some(ComponentRef::Str("middle")));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(2)));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(1)));
    assert_eq!(it.next_back(), Some(ComponentRef::Str("start")));
    assert_eq!(it.next_back(), None);
    Ok(())
}

#[test]
fn test_next_back_str() -> alloc::Result<()> {
    let mut item = ItemBuf::new();

    item.push(ComponentRef::Crate("std"))?;
    item.push("start")?;
    item.push(ComponentRef::Id(1))?;
    item.push(ComponentRef::Id(2))?;
    item.push("middle")?;
    item.push(ComponentRef::Id(3))?;
    item.push("end")?;

    let mut it = item.iter();

    assert_eq!(it.next_back_str(), Some("end"));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(3)));
    assert_eq!(it.next_back_str(), Some("middle"));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(2)));
    assert_eq!(it.next_back(), Some(ComponentRef::Id(1)));
    assert_eq!(it.next_back_str(), Some("start"));
    assert_eq!(it.next_back(), Some(ComponentRef::Crate("std")));
    assert_eq!(it.next_back(), None);
    Ok(())
}

#[test]
fn alternate() -> alloc::Result<()> {
    let mut item = ItemBuf::new();

    item.push(ComponentRef::Crate("std"))?;
    item.push("start")?;
    item.push(ComponentRef::Id(1))?;
    item.push(ComponentRef::Id(2))?;
    item.push("middle")?;
    item.push(ComponentRef::Id(3))?;
    item.push("end")?;

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
    Ok(())
}

#[test]
fn store_max_data() -> alloc::Result<()> {
    let mut item = ItemBuf::new();
    item.push(ComponentRef::Id(MAX_DATA - 1))?;
    assert_eq!(item.last(), Some(ComponentRef::Id(MAX_DATA - 1)));
    Ok(())
}

#[test]
fn store_max_string() -> alloc::Result<()> {
    let mut item = ItemBuf::new();
    let s = "x".repeat(MAX_DATA - 1);
    item.push(ComponentRef::Str(&s))?;
    assert_eq!(item.last(), Some(ComponentRef::Str(&s)));
    Ok(())
}

/// A component which does not fit is an error rather than a panic.
///
/// The size is a name out of a source file, so asserting on it turned any
/// script carrying a long enough identifier into an abort. It only did so in a
/// debug build, which is where a host runs its tests, and these two tests only
/// passed there for the same reason.
#[test]
fn store_max_data_overflow() {
    let mut item = ItemBuf::new();

    assert!(matches!(
        item.push(ComponentRef::Id(MAX_DATA)),
        Err(alloc::Error::CapacityOverflow)
    ));

    assert_eq!(item.last(), None);
}

#[test]
fn store_max_string_overflow() {
    let mut item = ItemBuf::new();
    let s = "x".repeat(MAX_DATA);

    assert!(matches!(
        item.push(ComponentRef::Str(&s)),
        Err(alloc::Error::CapacityOverflow)
    ));

    assert_eq!(item.last(), None);
}

#[test]
fn validate() -> alloc::Result<()> {
    assert!(internal::validate(b""));

    let mut item = ItemBuf::with_crate("std")?;
    item.push("option")?;
    item.push(ComponentRef::Id(42))?;
    item.push("Option")?;

    let bytes = item.as_bytes();
    assert!(internal::validate(bytes));

    // A strict prefix is only valid if it ends on a component boundary: the
    // empty item, and after each of the first three components.
    let valid_prefixes = (0..bytes.len())
        .filter(|&len| internal::validate(&bytes[..len]))
        .count();

    assert_eq!(valid_prefixes, 4);
    Ok(())
}

#[test]
fn validate_rejects_malformed() -> alloc::Result<()> {
    let item = ItemBuf::with_item(["name"])?;
    let bytes = item.as_bytes();
    let last = bytes.len() - 1;

    // Tail tag which does not match the head tag.
    let mut mismatched = bytes.to_vec();
    mismatched[last] ^= 0xff;
    assert!(!internal::validate(&mismatched));

    // String which is not UTF-8.
    let mut not_utf8 = bytes.to_vec();
    not_utf8[internal::TAG_BYTES] = 0xff;
    assert!(!internal::validate(&not_utf8));

    // Tag of an unknown type.
    assert!(!internal::validate(&0b11u16.to_ne_bytes()));

    // Half a tag.
    assert!(!internal::validate(&[0]));
    Ok(())
}
