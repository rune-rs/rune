use rune_tests::*;

#[test]
fn basic_use() {
    rune! {
        () =>
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
    }
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_use() {
    assert_compile_error! {
        r#"
        fn function() {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        }
        "#,
        span, NestedTest { nested_span } => {
            assert_eq!(span, Span::new(37, 69));
            assert_eq!(nested_span, Span::new(9, 22));
        }  
    }

    assert_compile_error! {
        r#"
        const ITEM = {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        };
        "#,
        span, NestedTest { nested_span } => {
            assert_eq!(span, Span::new(36, 68));
            assert_eq!(nested_span, Span::new(9, 19));
        }  
    }
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_bench() {
    assert_compile_error! {
        r#"
        fn function() {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        }
        "#,
        span, NestedBench { nested_span } => {
            assert_eq!(span, Span::new(37, 71));
            assert_eq!(nested_span, Span::new(9, 22));
        }  
    }

    assert_compile_error! {
        r#"
        const ITEM = {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        };
        "#,
        span, NestedBench { nested_span } => {
            assert_eq!(span, Span::new(36, 70));
            assert_eq!(nested_span, Span::new(9, 19));
        }  
    }
}
