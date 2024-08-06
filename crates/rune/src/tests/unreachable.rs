prelude!();

use WarningDiagnosticKind::*;

#[test]
fn unreachable_iter() {
    assert_warnings! {
        r#"
        pub fn function() {
            for _ in { return 10 } { 1 }
            2
        }
        "#,
        span,
        Unreachable { cause: span!(50, 63), .. } => {
            assert_eq!(span, span!(64, 69));
        },
        Unreachable { cause: span!(41, 69), .. } => {
            assert_eq!(span, span!(82, 83));
        },
    }
}
