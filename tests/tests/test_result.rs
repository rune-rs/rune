//! Test for result functions

use rune::VmErrorKind::*;
use rune_tests::*;

#[test]
fn test_map() {
    assert_eq!(
        rune! { Result<u32, String> =>
            pub fn main() {
                Ok(1).map(|v| v + 1)
            }
        },
        Ok(2)
    )
}

#[test]
fn test_and_then() {
    assert_eq!(
        rune! { Result<u32, String> =>

            pub fn main() {
                Ok(1).and_then(|v| Ok(v + 1))
            }
        },
        Ok(2)
    )
}

#[test]
fn test_and_then_error() {
    assert_eq!(
        rune! { Result<u32, String> =>

            pub fn main() {
                Ok(1).and_then(|v| Err("Failed"))
            }
        },
        Err("Failed".to_owned())
    )
}

#[test]
fn test_expect_some() {
    assert_eq!(
        rune! { i32 =>
            pub fn main() {
                Ok(1).expect("Ok")
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
            Err("Error").expect("Err('Error')")
       }
    "#,
        Panic { reason} => {
            assert_eq!(reason.to_string(),
                       "Err('Error'): \"Error\"")
        }
    );
}

#[test]
fn test_unwrap_some() {
    assert_eq!(
        rune! { i32 =>
            pub fn main() {
                Ok(1).unwrap()
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
            Err("Error").unwrap()
       }
    "#,
        Panic { reason } => {
            assert_eq!(reason.to_string(),
                       "called `Result::unwrap()` on an `Err` value: \"Error\"")
        }
    );
}

#[test]
fn test_unwrap_or() {
    assert_eq!(
        rune! { i32 =>
            pub fn main() {
                Err("Error").unwrap_or(10)
            }
        },
        10
    );
}
