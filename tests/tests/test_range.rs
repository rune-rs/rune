use rune_tests::*;

#[test]
fn test_range() {
    rune! { () =>
        pub fn main() {
            assert_eq!((1..10).start, Some(1));
            assert_eq!((1..10).end, Some(10));
            assert_eq!((1..).start, Some(1));
            assert_eq!((1..).end, None);
            assert_eq!((..10).start, None);
            assert_eq!((..10).end, Some(10));
            assert_eq!((..).start, None);
            assert_eq!((..).end, None);

            assert_eq!((1..=10).start, Some(1));
            assert_eq!((1..=10).end, Some(10));
            assert_eq!((1..=).start, Some(1));
            assert_eq!((1..=).end, None);
            assert_eq!((..=10).start, None);
            assert_eq!((..=10).end, Some(10));
            assert_eq!((..=).start, None);
            assert_eq!((..=).end, None);
        }
    };
}

#[test]
fn test_range_iter() {
    rune! { () =>
        pub fn main() {
            assert_eq!((1..4).iter().collect_vec(), [1, 2, 3]);
            assert_eq!((1..4).iter().rev().collect_vec(), [3, 2, 1]);
            assert_eq!((1..=4).iter().collect_vec(), [1, 2, 3, 4]);
            assert_eq!((1..=4).iter().rev().collect_vec(), [4, 3, 2, 1]);
            assert_eq!((0..10).iter().rev().take(3).collect_vec(), [9, 8, 7]);
            assert_eq!((0..).iter().take(3).collect_vec(), [0, 1, 2]);

            let n = 1;
            assert_eq!((n + 1..).iter().take(3).collect_vec(), [2, 3, 4]);
        }
    };
}

#[test]
fn test_non_numeric_ranges() {
    rune! { () =>
        pub fn main() {
            assert_eq!((#{}..=10).start, Some(#{}));
        }
    };

    rune! { () =>
        pub fn main() {
            let a = ..=(1, 2, 3);
            assert_eq!(a.start, None);
            assert_eq!(a.end, Some((1, 2, 3)));
        }
    };
}

#[test]
fn test_range_into_iter() {
    rune! { () =>
        pub fn main() {
            let d = [];

            for n in 0..=3 {
                d.push(n);
            }

            assert_eq!(d, [0, 1, 2, 3]);
        }
    };

    rune! {
        () =>
        pub fn main() {
            fn end() {
                4
            }

            let d = [];

            for n in 0 + 1..=end() {
                d.push(n);
            }

            assert_eq!(d, [1, 2, 3, 4]);
        }
    }
}

/// Ensures that the end of the range is parsed without an eager brace to ensure
/// it can be used in a loop.
#[test]
fn test_range_non_eager_brace() {
    let out = rune! {
        i64 =>
        pub fn main() {
            let out = 0;

            for n in 0..10 {
                out += n;
            }

            out
        }
    };

    let expected = (0i64..10i64).sum::<i64>();
    assert_eq!(out, expected);
}
