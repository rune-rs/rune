use super::Arena;

#[test]
fn basic() {
    let arena = Arena::new();

    let hello = arena.alloc(1u32).unwrap();
    let world = arena.alloc(2u32).unwrap();

    assert_eq!(hello, &1);
    assert_eq!(world, &2);
}
