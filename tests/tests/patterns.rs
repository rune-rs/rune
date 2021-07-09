use rune_tests::*;

#[test]
fn test_patterns() {
    assert_eq! {
        rune!(i64 => pub fn main() { match 1 { _ => 10 } }),
        10,
    };

    assert_eq! {
        rune!(i64 => pub fn main() { match 10 { n => 10 } }),
        10,
    };

    assert_eq! {
        rune!(char => pub fn main() { match 'a' { 'a' => 'b', n => n } }),
        'b',
    };

    assert_eq! {
        rune!(i64 => pub fn main() { match 10 { n => n } }),
        10,
    };

    assert_eq! {
        rune!(i64 => pub fn main() { match 10 { 10 => 5, n => n } }),
        5,
    };

    assert_eq! {
        rune!(String => pub fn main() { match "hello world" { "hello world" => "hello john", n => n } }),
        "hello john",
    };

    assert_eq! {
        rune!(String => pub fn main() { match (true, false) { (true, false) => "aaaa" , _ => "no", } }),
        "aaaa",
    };

    assert_eq! {
        rune!(String => pub fn main() { match (1==1, 1==2) { (true, false) => "aaaa" , _ => "no", } }),
        "aaaa",
    };
}

#[test]
fn test_vec_patterns() {
    assert! {
        rune!(bool => pub fn main() { match [] { [..] => true } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [] { [..] => true, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, 2] { [a, b] => a + 1 == b } }),
    };

    assert_eq! {
        rune!(() => pub fn main() { match [] { [a, b] => a + 1 == b } }),
        (),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, 2] { [a, b] => a + 1 == b, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, 2] { [a, b, ..] => a + 1 == b, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, 2] { [1, ..] => true, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match [1, 2] { [] => true, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, 2] { [1, 2] => true, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match [1, 2] { [1] => true, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, [2, 3]] { [1, [2, ..]] => true, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match [1, []] { [1, [2, ..]] => true, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match [1, [2, 3]] { [1, [2, 3]] => true, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match [1, [2, 4]] { [1, [2, 3]] => true, _ => false } }),
    };
}

#[test]
fn test_object_patterns() {
    assert! {
        rune!(bool => pub fn main() { match #{} { #{..} => true } }),
    };

    assert! {
        rune!(bool => pub fn main() { match #{foo: true} { #{foo} => foo, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match #{} { #{..} => true, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v, ..} => v == 10, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v} => v == 10, _ => false } }),
    };

    assert! {
        !rune!(bool => pub fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v} => v == 10, _ => false } }),
    };

    assert! {
        rune!(bool => pub fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v, ..} => v == 10, _ => false } }),
    };
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
