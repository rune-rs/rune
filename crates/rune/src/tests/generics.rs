prelude!();

use std::collections::HashMap;

#[test]
fn test_collect_vec() {
    let values: Vec<i64> = rune! {
        pub fn main() {
            let it = [4, 3, 2, 1].iter();
            it.collect::<Vec>()
        }
    };
    assert_eq!(values, vec![4, 3, 2, 1]);

    let values: Vec<i64> = rune! {
        use std::iter::Iterator;

        pub fn main() {
            let it = [4, 3, 2, 1].iter();
            Iterator::collect::<Vec>(it)
        }
    };
    assert_eq!(values, vec![4, 3, 2, 1]);

    let values: Vec<i64> = rune! {
        use std::iter::Iterator;

        pub fn main() {
            let it = [4, 3, 2, 1].iter();
            let c = Iterator::collect::<Vec>;
            c(it)
        }
    };
    assert_eq!(values, vec![4, 3, 2, 1]);
}

#[test]
fn test_collect_object() {
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

    let values: HashMap<String, i64> = rune! {
        use std::iter::Iterator;

        pub fn main() {
            let it = [("a", 4), ("b", 3), ("c", 2), ("d", 1)].iter();
            Iterator::collect::<Object>(it)
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

    let values: HashMap<String, i64> = rune! {
        use std::iter::Iterator;

        pub fn main() {
            let it = [("a", 4), ("b", 3), ("c", 2), ("d", 1)].iter();
            let c = Iterator::collect::<Object>;
            c(it)
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
            vec.sort();
            vec
        }
    };

    assert_eq!(values, vec![1, 2, 3, 4]);

    let values: Vec<i64> = rune! {
        pub fn main() {
            let vec = [4, 3, 2, 1];
            Vec::sort(vec);
            vec
        }
    };
    assert_eq!(values, vec![1, 2, 3, 4]);

    let values: Vec<i64> = rune! {
        pub fn main() {
            let vec = [4, 3, 2, 1];
            let f = Vec::sort;
            f(vec);
            vec
        }
    };
    assert_eq!(values, vec![1, 2, 3, 4]);
}
