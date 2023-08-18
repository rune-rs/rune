use super::Arena;

#[test]
fn basic() {
    let arena = Arena::new();

    let first = arena.alloc(1u32).unwrap();
    let second = arena.alloc(2u32).unwrap();

    assert_eq!(first, &1);
    assert_eq!(second, &2);
}

#[test]
fn slices() {
    let arena = Arena::new();

    let hello = arena.alloc_bytes(b"hello").unwrap();
    let world = arena.alloc_bytes(b"world").unwrap();

    assert_eq!(hello, b"hello");
    assert_eq!(world, b"world");
}
