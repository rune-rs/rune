prelude!();

use CompileErrorKind::*;

#[test]
fn basic_use() {
    let _: () = rune! {
        mod private {
            #[test]
            fn test_case() {
                assert_eq!(1 + 1, 2);
            }
        }

        #[test]
        fn test_case() {
            assert_eq!(1 + 1, 2);
        }

        pub fn main() {
        }
    };
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_use() {
    assert_errors! {
        r#"
        fn function() {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        }
        "#,
        span, NestedTest { nested_span } => {
            assert_eq!(span, span!(37, 69));
            assert_eq!(nested_span, span!(9, 22));
        }
    }

    assert_errors! {
        r#"
        const ITEM = {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        };
        "#,
        span!(36, 68), NestedTest { nested_span: span!(9, 19) }
    }
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_bench() {
    assert_errors! {
        r#"
        fn function() {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        }
        "#,
        span!(37, 71), NestedBench { nested_span: span!(9, 22) }
    }

    assert_errors! {
        r#"
        const ITEM = {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        };
        "#,
        span!(36, 70), NestedBench { nested_span: span!(9, 19) }
    }
}
