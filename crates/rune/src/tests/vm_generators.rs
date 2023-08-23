prelude!();

#[test]
fn test_simple_generator() {
    let out: i64 = rune! {
        fn foo() { yield 1; yield 2; yield 3; }

        pub fn main() {
            let gen = foo();
            let result = 0;

            while let Some(value) = gen.next() {
                result += value;
            }

            result
        }
    };
    assert_eq!(out, 6);
}

#[test]
fn test_resume() {
    let out: i64 = rune! {
        use std::ops::GeneratorState;

        fn foo() { let a = yield 1; let b = yield a; b }

        pub fn main() {
            let gen = foo();
            let result = 0;

            if let GeneratorState::Yielded(value) = gen.resume(()) {
                result += value;
            } else {
                panic("unexpected");
            }

            if let GeneratorState::Yielded(value) = gen.resume(2) {
                result += value;
            } else {
                panic("unexpected");
            }

            if let GeneratorState::Complete(value) = gen.resume(3) {
                result += value;
            } else {
                panic("unexpected");
            }

            result
        }
    };
    assert_eq!(out, 6);
}
