prelude!();

use VmErrorKind::*;

#[test]
fn test_map() {
    let out: Option<u32> = rune! {
        pub fn main() {
            Some(1).map(|v| v + 1)
        }
    };
    assert_eq!(out, Some(2))
}

#[test]
fn test_and_then() {
    let out: Option<i32> = rune! {
        pub fn main() {
            Some(1).and_then(|v| Some(v + 1))
        }
    };
    assert_eq!(out, Some(2))
}

#[test]
fn test_expect_some() {
    let out: i32 = rune! {
        pub fn main() {
            Some(1).expect("Some")
        }
    };
    assert_eq!(out, 1);
}

#[test]
fn test_expect() {
    assert_vm_error!(
        r#"
        pub fn main() {
            None.expect("None")
       }
    "#,
        Panic { reason} => {
            assert_eq!(reason.to_string(),
                       "None")
        }
    );
}

#[test]
fn test_unwrap_some() {
    let out: i32 = rune! {
        pub fn main() {
            Some(1).unwrap()
        }
    };
    assert_eq!(out, 1);
}

#[test]
fn test_unwrap() {
    assert_vm_error!(
        r#"
        pub fn main() {
            None.unwrap()
       }
    "#,
        Panic { reason} => {
            assert_eq!(reason.to_string(),
                       "Called `Option::unwrap()` on a `None` value")
        }
    );
}
