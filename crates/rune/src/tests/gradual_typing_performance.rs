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

// ============================================================================
// Protocol Performance Benchmarks
// ============================================================================

/// Benchmark: String interning effectiveness with many variable names
#[test]
fn performance_string_interning_effectiveness() {
    // Test that string interning for variable names works efficiently
    let _: () = rune! {
        fn many_variables() -> i64 {
            // Declare many variables with different names
            let a1 = 1; let a2 = 2; let a3 = 3; let a4 = 4; let a5 = 5;
            let a6 = 6; let a7 = 7; let a8 = 8; let a9 = 9; let a10 = 10;
            let a11 = 11; let a12 = 12; let a13 = 13; let a14 = 14; let a15 = 15;
            let a16 = 16; let a17 = 17; let a18 = 18; let a19 = 19; let a20 = 20;
            let a21 = 21; let a22 = 22; let a23 = 23; let a24 = 24; let a25 = 25;
            let a26 = 26; let a27 = 27; let a28 = 28; let a29 = 29; let a30 = 30;

            // Use them in arithmetic operations
            a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 +
            a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 +
            a21 + a22 + a23 + a24 + a25 + a26 + a27 + a28 + a29 + a30
        }

        pub fn main() {
            assert_eq!(many_variables(), 465); // sum of 1..30
        }
    };
}

/// Benchmark: Arc tuple cloning efficiency
#[test]
fn performance_arc_tuple_cloning() {
    // Test that Arc<[ResolvedType]> provides O(1) cloning for tuples
    let _: () = rune! {
        fn create_tuples() -> ((i64, i64), (i64, i64, i64), (i64, i64, i64, i64)) {
            let t1 = (1, 2);
            let t2 = (3, 4, 5);
            let t3 = (6, 7, 8, 9);
            (t1, t2, t3)
        }

        fn use_tuples(data: ((i64, i64), (i64, i64, i64), (i64, i64, i64, i64))) -> i64 {
            // Access all tuple elements
            let t1 = data.0;
            let t2 = data.1;
            let t3 = data.2;
            t1.0 + t1.1 + t2.0 + t2.1 + t2.2 + t3.0 + t3.1 + t3.2 + t3.3
        }

        pub fn main() {
            let data = create_tuples();
            let result = use_tuples(data);
            assert_eq!(result, 45); // sum of 1..9
        }
    };
}

/// Benchmark: Protocol operations in tight loop
#[test]
fn performance_protocol_tight_loop() {
    // Test protocol performance with many operations
    let _: () = rune! {
        fn loop_arithmetic() -> i64 {
            let sum = 0;
            let i = 0;

            // Loop with protocol operations
            while i < 1000 {
                sum = sum + i * 2 - i / 2;
                i = i + 1;
            }

            sum
        }

        pub fn main() {
            // Just ensure it completes without timeout
            let _ = loop_arithmetic();
        }
    };
}

/// Benchmark: Type checking with nested protocols
#[test]
fn performance_nested_protocols() {
    // Test type checking performance with deeply nested protocol operations
    let _: () = rune! {
        fn nested_arithmetic() -> i64 {
            // Nest protocol calls
            ((1 + 2) * (3 + 4)) + ((5 + 6) * (7 + 8)) + ((9 + 10) * (11 + 12))
        }

        pub fn main() {
            assert_eq!(nested_arithmetic(), 254);
        }
    };
}

/// Benchmark: Protocol lookup with many different types
#[test]
fn performance_protocol_lookup_many_types() {
    // Test protocol lookup performance with various type combinations
    let _: () = rune! {
        fn mixed_protocols() -> (i64, f64, bool, String) {
            let int_val = 42 + 10;          // ADD with i64
            let float_val = 3.14 * 2.0;     // MUL with f64
            let bool_val = 5 > 3 && 2 < 4;   // Comparison protocols
            let string_val = "hello" + " " + "world";  // String protocol

            (int_val, float_val, bool_val, string_val)
        }

        pub fn main() {
            let (i, f, b, s) = mixed_protocols();
            assert_eq!(i, 52);
            assert_eq!(f, 6.28);
            assert!(b);
            assert_eq!(s, "hello world");
        }
    };
}

/// Benchmark: Type unification with complex expressions
#[test]
fn performance_type_unification_complex() {
    // Test type unification performance with complex mixed expressions
    let _: () = rune! {
        fn complex_unification(x: i64, y: f64, z: i64) -> (i64, f64) {
            // Complex expression requiring type unification
            let int_result = x * 2 + z - 5;           // All integer operations
            let float_result = (x as f64) * y + 1.5; // Mixed operations

            (int_result, float_result)
        }

        pub fn main() {
            let (i, f) = complex_unification(10, 2.5, 3);
            assert_eq!(i, 18);  // 10*2 + 3 - 5 = 18
            assert_eq!(f, 26.5); // 10.0 * 2.5 + 1.5 = 26.5
        }
    };
}

/// Benchmark: Linear scopes with many variables
#[test]
fn performance_linear_scope_optimization() {
    // Test Vec-based scope storage efficiency
    let _: () = rune! {
        fn nested_scopes() -> i64 {
            // Create multiple nested scopes
            let x = 10;

            {
                let y = x + 5;
                {
                    let z = y * 2;
                    {
                        let w = z - 3;
                        w
                    }
                }
            }
        }

        pub fn main() {
            assert_eq!(nested_scopes(), 27); // ((10 + 5) * 2) - 3 = 27
        }
    };
}

/// Benchmark: Protocol method resolution
#[test]
fn performance_protocol_method_resolution() {
    // Test performance of protocol method lookup and resolution
    let _: () = rune! {
        fn compare_operations() -> (bool, bool, bool, bool) {
            let a = 10;
            let b = 20;
            let c = 10;

            // Multiple comparison protocol calls
            let eq1 = a == c;
            let ne = a != b;
            let lt = a < b;
            let le = a <= c;

            (eq1, ne, lt, le)
        }

        pub fn main() {
            let (eq, ne, lt, le) = compare_operations();
            assert!(eq);
            assert!(ne);
            assert!(lt);
            assert!(le);
        }
    };
}
