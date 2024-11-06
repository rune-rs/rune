//! Test for result functions

prelude!();

use VmErrorKind::*;

#[test]
fn panics() {
    assert_vm_error!(
        "Err(\"Error\").expect(\"Err('Error')\")",
        Panic { reason } => {
            assert_eq!(reason.to_string(), "Err('Error'): \"Error\"")
        }
    );

    assert_vm_error!(
        "Err(\"Error\").unwrap()",
        Panic { reason } => {
            assert_eq!(reason.to_string(), "Called `Result::unwrap()` on an `Err` value: \"Error\"")
        }
    );
}
