use rune::compile::CompileErrorKind::*;
use rune::span;
use rune_tests::*;

#[test]
fn test_closure_moved() {
    assert_compile_error!(
        r#"
        pub fn main() {
            let o = [];
            let a = move || {
                o.push(42);
                o
            };
        
            o.push(42);
            a()
        }
        "#,
        span, VariableMoved { moved_at } => {
            assert_eq!(span, span!(161, 162));
            assert_eq!(moved_at, span!(69, 138));
        }
    )
}

#[test]
fn test_async_moved() {
    assert_compile_error!(
        r#"
        pub async fn main() {
            let o = [];
            let a = async move {
                o.push(42);
                o
            };

            o.push(42);
            a.await
        }
        "#,
        span, VariableMoved { moved_at } => {
            assert_eq!(span, span!(162, 163));
            assert_eq!(moved_at, span!(75, 147));
        }
    )
}
