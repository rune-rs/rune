//! Test for result functions

prelude!();

use VmErrorKind::*;

#[test]
fn test_map() {
    let out: Result<u32, String> = rune! {
        pub fn main() {
            Ok(1).map(|v| v + 1)
        }
    };
    assert_eq!(out, Ok(2))
}

#[test]
fn test_and_then() {
    let out: Result<u32, String> = rune! {
        pub fn main() {
            Ok(1).and_then(|v| Ok(v + 1))
        }
    };
    assert_eq!(out, Ok(2))
}

#[test]
fn test_and_then_error() {
    let out: Result<u32, String> = rune! {

        pub fn main() {
            Ok(1).and_then(|v| Err("Failed"))
        }
    };
    assert_eq!(out, Err("Failed".to_owned()))
}

#[test]
fn test_expect_some() {
    let out: i32 = rune! {
        pub fn main() {
            Ok(1).expect("Ok")
        }
    };
    assert_eq!(out, 1);
}

#[test]
fn test_expect() {
    assert_vm_error!(r#"
        pub fn main() {
            Err("Error").expect("Err('Error')")
       }
    "#,
        Panic { reason } => {
            assert_eq!(reason.to_string(), "Err('Error'): \"Error\"")
        }
    );
}

#[test]
fn test_unwrap_some() {
    let out: i32 = rune! {
        pub fn main() {
            Ok(1).unwrap()
        }
    };
    assert_eq!(out, 1);
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
                       "Called `Result::unwrap()` on an `Err` value: \"Error\"")
        }
    );
}

#[test]
fn test_unwrap_or() {
    let out: i32 = rune! {
        pub fn main() {
            Err("Error").unwrap_or(10)
        }
    };
    assert_eq!(out, 10);
}
