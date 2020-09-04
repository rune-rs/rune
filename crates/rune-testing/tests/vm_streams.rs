use rune_testing::*;

#[test]
fn test_simple_stream() {
    assert_eq! {
        rune! {
            i64 => r#"
            async fn foo() {
                yield 1;
                yield 2;
                yield 3;
            }

            async fn main() {
                let gen = foo();
                let result = 0;

                while let Some(value) = gen.next().await {
                    result += value;
                }

                result
            }
            "#
        },
        6,
    };
}

#[test]
fn test_resume() {
    assert_eq! {
        rune! {
            i64 => r#"
            use std::generator::GeneratorState;

            async fn foo() { let a = yield 1; let b = yield a; b }
            
            async fn main() {
                let gen = foo();
                let result = 0;
            
                if let GeneratorState::Yielded(value) = gen.resume(()).await {
                    result += value;
                } else {
                    panic("unexpected");
                }
            
                if let GeneratorState::Yielded(value) = gen.resume(2).await {
                    result += value;
                } else {
                    panic("unexpected");
                }
            
                if let GeneratorState::Complete(value) = gen.resume(3).await {
                    result += value;
                } else {
                    panic("unexpected");
                }
            
                result
            }
            "#
        },
        6,
    };
}
