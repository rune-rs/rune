//! Performance tests for gradual typing implementation.
//!
//! Key performance scenarios to ensure the type system scales reasonably.

prelude!();

/// Test function with moderate number of parameters (10 params)
#[test]
fn performance_many_parameters() {
    // Test moderate parameter count - ensures we handle typical functions
    let result: i64 = rune! {
        fn ten_parameters(p0: i64, p1: i64, p2: i64, p3: i64, p4: i64, p5: i64, p6: i64, p7: i64, p8: i64, p9: i64) -> i64 {
            p0 + p1 + p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9
        }

        ten_parameters(0, 1, 2, 3, 4, 5, 6, 7, 8, 9)
    };

    assert_eq!(result, 45); // sum of 0..9
}

/// Test function with nested tuples (moderate depth)
#[test]
fn performance_nested_tuples() {
    // Moderate nesting - ensures recursion handling works
    let _: () = rune! {
        pub fn main() {
            let x = (((42,),),);
            let y = x.0;
            let z = y.0;
            assert_eq!(z.0, 42);
        }
    };
}

/// Test struct with moderate number of fields (10 fields)
#[test]
fn performance_struct_fields() {
    let _: () = rune! {
        struct ManyFields {
            f0: i64, f1: i64, f2: i64, f3: i64, f4: i64,
            f5: i64, f6: i64, f7: i64, f8: i64, f9: i64,
        }

        pub fn main() {
            let s = ManyFields {
                f0: 0, f1: 1, f2: 2, f3: 3, f4: 4,
                f5: 5, f6: 6, f7: 7, f8: 8, f9: 9,
            };
            assert_eq!(s.f9, 9);
        }
    };
}

/// Test EXTREME: function with 20 parameters (stress test)
#[test]
fn performance_extreme_many_parameters() {
    // Test extreme parameter count - ensures we handle stress cases
    let result: i64 = rune! {
        fn many_parameters(p0: i64, p1: i64, p2: i64, p3: i64, p4: i64, p5: i64, p6: i64, p7: i64, p8: i64, p9: i64, p10: i64, p11: i64, p12: i64, p13: i64, p14: i64, p15: i64, p16: i64, p17: i64, p18: i64, p19: i64) -> i64 {
            p0 + p1 + p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9 + p10 + p11 + p12 + p13 + p14 + p15 + p16 + p17 + p18 + p19
        }

        many_parameters(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19)
    };

    assert_eq!(result, 190); // sum of 0..19
}

/// Test EXTREME: struct with 20 fields (stress test)
#[test]
fn performance_extreme_struct_fields() {
    let _: () = rune! {
        struct ManyFields {
            f0: i64, f1: i64, f2: i64, f3: i64, f4: i64,
            f5: i64, f6: i64, f7: i64, f8: i64, f9: i64,
            f10: i64, f11: i64, f12: i64, f13: i64, f14: i64,
            f15: i64, f16: i64, f17: i64, f18: i64, f19: i64,
        }

        pub fn main() {
            let s = ManyFields {
                f0: 0, f1: 1, f2: 2, f3: 3, f4: 4,
                f5: 5, f6: 6, f7: 7, f8: 8, f9: 9,
                f10: 10, f11: 11, f12: 12, f13: 13, f14: 14,
                f15: 15, f16: 16, f17: 17, f18: 18, f19: 19,
            };
            assert_eq!(s.f19, 19);
        }
    };
}

/// Test EXTREME: deeply nested tuples (tests recursion limit)
#[test]
fn performance_extreme_deep_nesting() {
    // Very deep nesting - tests recursion depth limit
    let _: () = rune! {
        pub fn main() {
            let x = ((((((42,),),),),),);
            let a = x.0;
            let b = a.0;
            let c = b.0;
            let d = c.0;
            let e = d.0;
            assert_eq!(e.0, 42);
        }
    };
}
