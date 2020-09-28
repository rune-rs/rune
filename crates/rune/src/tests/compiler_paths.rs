use crate::testing::*;

#[test]
fn test_paths_cannot_contain_crate() {
    assert_compile_error! {
        r#"fn main() { use crate::x::y::z; } "#,
        span, Internal { message } => {
            assert_eq!(message, "paths containing `crate` or `super` are not supported");
            assert_eq!(span, Span::new(16, 21));
        }
    };
}

#[test]
fn test_paths_cannot_contain_super() {
    assert_compile_error! {
        r#"fn main() { use super::x; } "#,
        span, Internal { message } => {
            assert_eq!(message, "paths containing `crate` or `super` are not supported");
            assert_eq!(span, Span::new(16, 21));
        }
    };
}
