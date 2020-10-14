use rune::testing::*;

#[test]
fn test_range_iter() {
    let values = rune!(Vec<i64> =>
        use std::iter::range;

        pub fn main() {
            range(0, 100).map(|n| n * 2).filter(|n| n % 3 == 0).collect_vec()
        }
    );

    assert_eq!(
        values,
        (0..100)
            .map(|n| n * 2)
            .filter(|n| n % 3 == 0)
            .collect::<Vec<i64>>()
    );
}

#[test]
fn test_rev() {
    let values = rune!(Vec<i64> =>
        use std::iter::range;

        pub fn main() {
            range(0, 100).map(|n| n * 2).filter(|n| n % 3 == 0).rev().collect_vec()
        }
    );

    assert_eq!(
        values,
        (0..100)
            .map(|n| n * 2)
            .filter(|n| n % 3 == 0)
            .rev()
            .collect::<Vec<i64>>()
    );
}

#[test]
fn test_next_back() {
    rune! {() =>
        const SOURCE = [1, 2, 3, "foo"];

        pub fn main() {
            let it = SOURCE.iter().rev();
            let v = Vec::new();

            while let Some(n) = it.next_back() {
                v.push(n);
            }

            assert_eq!(v, SOURCE);
        }
    };
}

#[test]
fn test_object_rev_error() {
    assert_vm_error!(
        r#"pub fn main() { #{}.iter().rev() }"#,
        Panic { reason } => {
            assert_eq!(reason.to_string(), "`std::object::Iter` is not a double-ended iterator");
        }
    );
}

#[test]
fn test_chain() {
    let values = rune! { Vec<i64> =>
        pub fn main() {
            [1, 2].iter().rev().chain([3, 4].iter()).collect_vec()
        }
    };

    assert_eq!(values, vec![2, 1, 3, 4])
}

#[test]
fn test_enumerate() {
    let values = rune! { Vec<(i64, i64)> =>
        pub fn main() {
            let it = [1, 2].iter().rev().chain([3, 4].iter()).enumerate();
            assert_eq!(it.next_back(), Some((3, 4)));
            it.collect_vec()
        }
    };

    assert_eq!(values, vec![(0, 2), (1, 1), (2, 3)])
}

#[test]
fn test_option_iter() {
    let values = rune! { Vec<i64> =>
        pub fn main() {
            Some(1).iter().chain(None.iter()).chain(Some(3).iter()).collect_vec()
        }
    };

    assert_eq!(values, vec![1, 3])
}

#[test]
fn test_peekable_take() {
    let actual = rune! { Vec<i64> =>
        use std::iter::range;

        pub fn main() {
            let it = range(1, 100).take(40).peekable();
            let out = [];

            while let Some(n) = it.next() {
                out.push(n);

                if it.peek().is_some() {
                    out.push(0);
                }
            }

            out
        }
    };

    let mut it = (1..100).take(40).peekable();
    let mut expected = Vec::new();

    while let Some(n) = it.next() {
        expected.push(n);
        expected.extend(it.peek().map(|_| 0));
    }

    assert_eq!(actual, expected);
}
