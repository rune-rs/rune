//! Test for result functions

prelude!();

use VmErrorKind::*;

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
