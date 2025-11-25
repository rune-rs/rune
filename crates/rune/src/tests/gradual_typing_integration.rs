//! Integration tests for gradual typing with core language features.

prelude!();

/// Typed async functions work correctly
#[test]
fn integrate_typed_async() {
    let result: i64 = rune! {
        async fn add(a: i64, b: i64) -> i64 {
            a + b
        }

        let future = add(5, 3);
        future.await
    };
    assert_eq!(result, 8);
}

/// Closures with type annotations
#[test]
fn integrate_typed_closures() {
    let result: i64 = rune! {
        let add = |a: i64, b: i64| -> i64 { a + b };
        add(10, 32)
    };
    assert_eq!(result, 42);
}

/// Match expressions with typed arms
#[test]
fn integrate_match_with_types() {
    let result: i64 = rune! {
        fn process(opt: Option) -> i64 {
            match opt {
                Some(x) => x,
                None => 0,
            }
        }

        process(Some(42))
    };
    assert_eq!(result, 42);
}

/// Complex integration: async + closures + structs
#[test]
fn integrate_complex_combination() {
    let result: i64 = rune! {
        struct Config {
            value: i64,
        }

        async fn process(config: Config) -> i64 {
            let transformer = |x: i64| -> i64 { x * 2 };
            transformer(config.value)
        }

        let cfg = Config { value: 21 };
        let future = process(cfg);
        future.await
    };
    assert_eq!(result, 42);
}
