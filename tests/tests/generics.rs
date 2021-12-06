use rune_tests::*;
use std::collections::HashMap;

#[test]
fn test_collect() {
    let values: Vec<i64> = rune! {
        pub fn main() {
            let it = [4, 3, 2, 1].iter();
            it.collect::<Vec>()
        }
    };
    assert_eq!(values, vec![4, 3, 2, 1]);

    let values: HashMap<String, i64> = rune! {
        pub fn main() {
            let it = [("a", 4), ("b", 3), ("c", 2), ("d", 1)].iter();
            it.collect::<Object>()
        }
    };
    let expected = [
        (String::from("a"), 4),
        (String::from("b"), 3),
        (String::from("c"), 2),
        (String::from("d"), 1),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();
    assert_eq!(values, expected);
}

#[test]
fn test_sort() {
    let values: Vec<i64> = rune! {
        pub fn main() {
            let vec = [4, 3, 2, 1];
            vec.sort::<int>();
            vec
        }
    };
    assert_eq!(values, vec![1, 2, 3, 4]);
}
