prelude!();

use ErrorKind::*;
use VmErrorKind::*;

#[test]
fn range_accessors() {
    rune! {
        pub fn main() {
            assert_eq!((1..10).start, 1);
            assert_eq!((1..10).end, 10);
            assert_eq!((1..).start, 1);
            assert_eq!((..10).end, 10);
            assert_eq!((1..=10).start, 1);
            assert_eq!((1..=10).end, 10);
            assert_eq!((1..).start, 1);
            assert_eq!((..=10).end, 10);
        }
    };
}

#[test]
fn range_iter() {
    rune! {
        pub fn main() {
            assert_eq!((1..4).iter().collect::<Vec>(), [1, 2, 3]);
            assert_eq!((1..4).iter().rev().collect::<Vec>(), [3, 2, 1]);
            assert_eq!((1..=4).iter().collect::<Vec>(), [1, 2, 3, 4]);
            assert_eq!((1..=4).iter().rev().collect::<Vec>(), [4, 3, 2, 1]);
            assert_eq!((0..10).iter().rev().take(3).collect::<Vec>(), [9, 8, 7]);
            assert_eq!((0..).iter().take(3).collect::<Vec>(), [0, 1, 2]);

            let n = 1;
            assert_eq!((n + 1..).iter().take(3).collect::<Vec>(), [2, 3, 4]);
        }
    };
}

#[test]
fn range_match() {
    rune! {
        pub fn main() {
            use std::ops::{RangeFrom, RangeFull, RangeInclusive, RangeToInclusive, RangeTo, Range};

            assert_eq!(match 2.. { RangeFrom { start } => start, _ => 1 }, 2);
            assert_eq!(match 2.. { RangeFrom { .. } => 2, _ => 1 }, 2);
            assert_eq!(match .. { RangeFull => 2, _ => 1 }, 2);
            assert_eq!(match .. { RangeFull { .. } => 2, _ => 1 }, 2);
            assert_eq!(match 2..=5 { RangeInclusive { start, end } => start + end, _ => 1 }, 7);
            assert_eq!(match 2..=5 { RangeInclusive { start, .. } => start, _ => 1 }, 2);
            assert_eq!(match ..=2 { RangeToInclusive { end } => end, _ => 1 }, 2);
            assert_eq!(match ..=2 { RangeToInclusive { .. } => 2, _ => 1 }, 2);
            assert_eq!(match ..2 { RangeTo { end } => end, _ => 1 }, 2);
            assert_eq!(match ..2 { RangeTo { .. } => 2, _ => 1 }, 2);
            assert_eq!(match 2..5 { Range { start, end } => start + end, _ => 1 }, 7);
            assert_eq!(match 2..5 { Range { start, .. } => start, _ => 1 }, 2);
        }
    };
}

#[test]
fn test_non_numeric_ranges() {
    rune! {
        pub fn main() {
            assert_eq!((#{}..=10).start, #{});
        }
    };

    rune! {
        pub fn main() {
            let a = ..=(1, 2, 3);
            assert_eq!(a.end, (1, 2, 3));
        }
    };
}

#[test]
fn test_range_into_iter() {
    rune! {
        pub fn main() {
            let d = [];

            for n in 0..=3 {
                d.push(n);
            }

            assert_eq!(d, [0, 1, 2, 3]);
        }
    };

    rune! {
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
    };
}

/// Ensures that the end of the range is parsed without an eager brace to ensure
/// it can be used in a loop.
#[test]
fn test_range_non_eager_brace() {
    let out: i64 = rune! {
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

#[test]
fn unsupported_compile_range() {
    assert_errors! {
        "pub fn main() { 'a'..= }",
        span!(16, 22), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported range, you probably want `..` instead of `..=`")
        }
    };

    assert_errors! {
        "pub fn main() { ..= }",
        span!(16, 19), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported range, you probably want `..` instead of `..=`")
        }
    };
}

#[test]
fn unsupported_iter_range() {
    assert_vm_error!(
        r#"pub fn main() { (1.0..).iter() }"#,
        UnsupportedIterRangeFrom { start } => {
            assert_eq!(start, f64::type_info());
        }
    );

    assert_vm_error!(
        r#"pub fn main() { (1.0..2.0).iter() }"#,
        UnsupportedIterRange { start, end } => {
            assert_eq!(start, f64::type_info());
            assert_eq!(end, f64::type_info());
        }
    );

    assert_vm_error!(
        r#"pub fn main() { (1.0..=2.0).iter() }"#,
        UnsupportedIterRangeInclusive { start, end } => {
            assert_eq!(start, f64::type_info());
            assert_eq!(end, f64::type_info());
        }
    );

    assert_vm_error!(
        r#"pub fn main() { for _ in 1.0.. {} }"#,
        UnsupportedIterRangeFrom { start } => {
            assert_eq!(start, f64::type_info());
        }
    );
}
