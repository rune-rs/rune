prelude!();

#[test]
fn test_simple_stream() {
    let out: i64 = rune! {
        async fn foo() {
            let n = 0;

            let give = || {
                n + 1
            };

            yield give();
            yield give();
            yield give();
        }

        pub async fn main() {
            let gen = foo();
            let result = 0;

            while let Some(value) = gen.next().await {
                result += value;
            }

            result
        }
    };
    assert_eq!(out, 3);
}

#[test]
fn test_resume() {
    let out: i64 = rune! {
        use std::ops::GeneratorState;

        async fn foo() { let a = yield 1; let b = yield a; b }

        pub async fn main() {
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
    };
    assert_eq!(out, 6);
}
