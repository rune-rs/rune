prelude!();

use ErrorKind::*;
use VmErrorKind::*;

#[test]
fn test_patterns() {
    let out: i64 = rune!(
        pub fn main() {
            match 1 {
                _ => 10,
            }
        }
    );
    assert_eq!(out, 10);

    let out: i64 = rune!(
        pub fn main() {
            match 10 {
                n => 10,
            }
        }
    );
    assert_eq!(out, 10);

    let out: char = rune!(
        pub fn main() {
            match 'a' {
                'a' => 'b',
                n => n,
            }
        }
    );
    assert_eq!(out, 'b');

    let out: i64 = rune!(
        pub fn main() {
            match 10 {
                n => n,
            }
        }
    );
    assert_eq!(out, 10);

    let out: i64 = rune!(
        pub fn main() {
            match 10 {
                10 => 5,
                n => n,
            }
        }
    );
    assert_eq!(out, 5);

    let out: String = rune!(
        pub fn main() {
            match "hello world" {
                "hello world" => "hello john",
                n => n,
            }
        }
    );
    assert_eq!(out, "hello john");

    let out: String = rune!(
        pub fn main() {
            match (true, false) {
                (true, false) => "aaaa",
                _ => "no",
            }
        }
    );
    assert_eq!(out, "aaaa");

    let out: String = rune!(
        pub fn main() {
            match (1 == 1, 1 == 2) {
                (true, false) => "aaaa",
                _ => "no",
            }
        }
    );
    assert_eq!(out, "aaaa");
}

#[test]
fn test_vec_patterns() {
    let out: bool = rune!(
        pub fn main() {
            match [] {
                [..] => true,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [] {
                [..] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [a, b] => a + 1 == b,
            }
        }
    );
    assert_eq!(out, true);

    rune!(
        pub fn main() {
            match [] {
                [a, b] => a + 1 == b,
            }
        }
    );

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [a, b] => a + 1 == b,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [a, b, ..] => a + 1 == b,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [1, ..] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [1, 2] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, 2] {
                [1] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            match [1, [2, 3]] {
                [1, [2, ..]] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, []] {
                [1, [2, ..]] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            match [1, [2, 3]] {
                [1, [2, 3]] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            match [1, [2, 4]] {
                [1, [2, 3]] => true,
                _ => false,
            }
        }
    );
    assert_eq!(out, false);
}

#[test]
fn test_object_patterns() {
    let out: bool = rune!(pub fn main() { match #{} { #{..} => true } });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { match #{foo: true} { #{foo} => foo, _ => false } });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { match #{} { #{..} => true, _ => false } });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v, ..} => v == 10, _ => false } });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v} => v == 10, _ => false } });
    assert_eq!(out, false);

    let out: bool = rune!(pub fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v} => v == 10, _ => false } });
    assert_eq!(out, false);

    let out: bool = rune!(pub fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v, ..} => v == 10, _ => false } });
    assert_eq!(out, true);
}

#[test]
fn test_bad_pattern() {
    // Attempting to assign to an unmatched pattern leads to a panic.
    assert_vm_error!(
        r#"
        pub fn main() {
            let [] = [1, 2, 3];
        }
        "#,
        Panic { reason } => {
            assert_eq!(reason.to_string(), "pattern did not match");
        }
    );
}

#[test]
fn test_const_in_pattern() {
    macro_rules! test_case_s {
        ($pat1:expr, $pat2:expr) => {
            let string = format! {
                r#"
                const PAT1 = {pat1};
                const PAT2 = {pat2};
                const PAT3 = {{
                    if true {{
                        {pat2}
                    }} else {{
                        {pat1}
                    }}
                }};

                pub fn main() {{
                    let value = {pat1};
                    let a = match value {{ PAT => 1, _ => 5 }};
                    let b = match value {{ PAT2 => 2, _ => 6 }};
                    let c = match value {{ PAT3 => 3, _ => 7 }};
                    let d = match value {{ PAT4 => 4, _ => 8 }};
                    (a, b, c, d)
                }}
                "#,
                pat1 = $pat1,
                pat2 = $pat2,
            };

            let tuple: (i64, i64, i64, i64) = rune_s!(string.as_str());
            assert_eq!(tuple, (1, 6, 7, 4));
        };
    }

    macro_rules! test_case {
        ($pat1:expr, $pat2:expr) => {
            test_case_s!(stringify!($pat1), stringify!($pat2))
        };
    }

    test_case!(true, false);
    test_case!('a', 'b');
    test_case!(b'a', b'b');
    test_case!(10, 20);
    test_case!("abc", "def");
    test_case!(b"abc", b"def");
    test_case!((1, 2), (3, 4));
    test_case!([1, 2], [3, 4]);
    test_case!([1, (3, 4)], [3, (3, 4)]);
    test_case_s!("#{foo: 12}", "#{bar: 12}");
    test_case_s!("#{}", "#{bar: 12}");

    assert_errors! {
        r#"
        const PAT = 3.1415;

        pub fn main() {
            let value = 3.1415;
            match value { PAT => true, _ => false }
        }
        "#,
        span!(112, 115), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported constant value in pattern");
        }
    }
}
