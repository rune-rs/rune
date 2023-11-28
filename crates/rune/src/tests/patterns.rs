prelude!();

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
