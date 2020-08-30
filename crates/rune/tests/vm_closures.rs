use rune_testing::*;

#[test]
fn test_nested_closures() {
    assert_eq! {
        4,
        test! {
            i64 => r#"
            fn main() {
                let var = 1;
            
                let a = |i| {
                    let b = |j| {
                        var + j
                    };
            
                    b(i + 1)
                };
            
                a(2)
            }
            "#
        }
    };
}

#[test]
fn test_closure_in_loop_iter() {
    assert_eq! {
        10,
        test! {
            i64 => r#"
            fn main() {
                let out = 1;

                for var in {
                    let a = || [1, 2, 3];
                    a()
                } {
                    let b = |n| var + n;
                    out += b(1);
                }

                out
            }
            "#
        }
    };
}

#[test]
fn test_capture_match() {
    assert_eq! {
        3,
        test! {
            i64 => r#"
            fn main() {
                let n = 1;
            
                let a = match { let out = || Some(n + 1); out() } {
                    Some(n) => |e| n + e,
                    _ => |_| 0,
                };
            
                a(1)
            }
            "#
        }
    };
}

#[test]
fn test_capture_fn_arg() {
    assert_eq! {
        3,
        test! {
            i64 => r#"
            fn foo(n) { |a| n + a }
            fn main() { foo(1)(2) }
            "#
        }
    };
}
