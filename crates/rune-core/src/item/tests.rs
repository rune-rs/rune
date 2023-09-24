use crate::item::internal::MAX_DATA;
use crate::item::{Component, ComponentRef, IntoComponent, ItemBuf};
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

    assert_eq!(item.pop()?, Some("end".into_component()?));
    assert_eq!(item.pop()?, Some(Component::Id(3)));
    assert_eq!(item.pop()?, Some("middle".into_component()?));
    assert_eq!(item.pop()?, Some(Component::Id(2)));
    assert_eq!(item.pop()?, Some(Component::Id(1)));
    assert_eq!(item.pop()?, Some("start".into_component()?));
    assert_eq!(item.pop()?, None);

    assert!(item.is_empty());
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

#[test]
#[should_panic(expected = "item data overflow, index or string size larger than MAX_DATA")]
fn store_max_data_overflow() {
    let mut item = ItemBuf::new();
    item.push(ComponentRef::Id(MAX_DATA)).unwrap();
    assert_eq!(item.last(), Some(ComponentRef::Id(MAX_DATA)));
}

#[test]
#[should_panic(expected = "item data overflow, index or string size larger than MAX_DATA")]
fn store_max_string_overflow() {
    let mut item = ItemBuf::new();
    let s = "x".repeat(MAX_DATA);
    item.push(ComponentRef::Str(&s)).unwrap();
}
