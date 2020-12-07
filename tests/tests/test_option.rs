//! Test for option functions

use rune_tests::*;

#[test]
fn test_map() {
    assert_eq!(
        rune! { Option<u32> =>
            pub fn main() {
                Some(1).map(|v| v + 1)
            }
        },
        Some(2)
    )
}

#[test]
fn test_and_then() {
    assert_eq!(
        rune! { Option<i32> =>

            pub fn main() {
                Some(1).and_then(|v| Some(v + 1))
            }
        },
        Some(2)
    )
}

#[test]
fn test_expect_some() {
    assert_eq!(
        rune! { i32 =>
            pub fn main() {
                Some(1).expect("Some")
            }
        },
        1
    );
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
    assert_eq!(
        rune! { i32 =>
            pub fn main() {
                Some(1).unwrap()
            }
        },
        1
    );
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
                       "called `Option::unwrap()` on a `None` value")
        }
    );
}
