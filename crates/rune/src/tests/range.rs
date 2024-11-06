prelude!();

use ErrorKind::*;
use VmErrorKind::*;

#[test]
fn unsupported_compile_range() {
    assert_errors! {
        "'a'..=",
        span!(0, 6), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported range, you probably want `..` instead of `..=`")
        }
    };

    assert_errors! {
        "..=",
        span!(0, 3), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported range, you probably want `..` instead of `..=`")
        }
    };
}

#[test]
fn unsupported_iter_range() {
    assert_vm_error!(
        "(1.0..).iter()",
        UnsupportedIterRangeFrom { start } => {
            assert_eq!(start, f64::type_info());
        }
    );

    assert_vm_error!(
        "(1.0..2.0).iter()",
        UnsupportedIterRange { start, end } => {
            assert_eq!(start, f64::type_info());
            assert_eq!(end, f64::type_info());
        }
    );

    assert_vm_error!(
        "(1.0..=2.0).iter()",
        UnsupportedIterRangeInclusive { start, end } => {
            assert_eq!(start, f64::type_info());
            assert_eq!(end, f64::type_info());
        }
    );

    assert_vm_error!(
        "for _ in 1.0..{}",
        UnsupportedIterRangeFrom { start } => {
            assert_eq!(start, f64::type_info());
        }
    );
}
