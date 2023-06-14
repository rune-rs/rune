prelude!();

use ErrorKind::*;

#[test]
fn test_closure_moved() {
    assert_errors!(
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
        span!(161, 162),
        VariableMoved {
            moved_at: span!(69, 138)
        }
    )
}

#[test]
fn test_async_moved() {
    assert_errors!(
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
        span!(162, 163),
        VariableMoved {
            moved_at: span!(75, 147)
        }
    )
}
