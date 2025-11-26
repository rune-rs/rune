//! Tests for complex gradual typing scenarios.
//!
//! These tests verify that type checking works correctly in more complex,
//! real-world usage patterns.

prelude!();

// ============================================================================
// Builder Pattern with Type Checking
// ============================================================================

/// Builder pattern with type annotations
#[test]
fn builder_pattern_with_types() {
    let result: i64 = rune! {
        struct QueryBuilder {
            _limit: i64,
            _offset: i64,
        }

        impl QueryBuilder {
            fn limit(self, limit: i64) -> QueryBuilder {
                QueryBuilder {
                    _limit: limit,
                    _offset: self._offset
                }
            }

            fn offset(self, offset: i64) -> QueryBuilder {
                QueryBuilder {
                    _limit: self._limit,
                    _offset: offset
                }
            }

            fn count(self) -> i64 {
                self._limit + self._offset
            }
        }

        fn build_query() -> i64 {
            let builder = QueryBuilder { _limit: 0, _offset: 0 };
            let result = builder
                .limit(100)
                .offset(50)
                .count();
            result
        }

        build_query()
    };
    assert_eq!(result, 150);
}

// ============================================================================
// Collection Pipeline with Type Preservation
// ============================================================================

/// Iterator-like operations with type annotations
#[test]
fn collection_pipeline_typed() {
    let result: i64 = rune! {
        fn process_numbers(nums: Vec) -> i64 {
            // Simulate filter and map operations
            let sum = 0;
            let count = 0;

            for num in nums {
                // Type checker knows num is i64 from Vec context
                if num > 0 {
                    sum = sum + num;
                    count = count + 1;
                }
            }

            sum / count  // Integer division
        }

        process_numbers([10, -5, 20, 0, 30])
    };
    assert_eq!(result, 20);  // (10 + 20 + 30) / 3 = 20
}

// ============================================================================
// Nested Function Calls with Protocol Types
// ============================================================================

/// Protocol operations through multiple function calls
#[test]
fn nested_protocol_calls() {
    let result: i64 = rune! {
        fn calculate(x: i64) -> i64 {
            x * 2
        }

        fn transform(x: i64) -> i64 {
            calculate(x) + 10
        }

        fn process(value: i64) -> i64 {
            let step1 = transform(value);
            let step2 = calculate(step1);
            step2 / 2
        }

        process(5)
    };
    assert_eq!(result, 20);  // ((5 * 2 + 10) * 2) / 2 = 20
}

// ============================================================================
// Complex Conditional Logic with Types
// ============================================================================

/// Complex if/else with type unification
#[test]
fn complex_conditional_types() {
    let result: i64 = rune! {
        fn calculate(a: i64, b: i64, c: i64) -> i64 {
            if a > 0 {
                if b > a {
                    b + c
                } else {
                    a + c
                }
            } else {
                c
            }
        }

        calculate(5, 3, 10)
    };
    assert_eq!(result, 15);  // 5 > 0 and 3 <= 5, so a + c = 5 + 10
}

// ============================================================================
// Recursive-like Patterns (Limited due to no true recursion)
// ============================================================================

/// Simulated recursion through iteration
#[test]
fn iterative_recursive_pattern() {
    let result: i64 = rune! {
        fn factorial(n: i64) -> i64 {
            let result = 1;
            let current = 1;

            while current <= n {
                result = result * current;
                current = current + 1;
            }

            result
        }

        factorial(5)
    };
    assert_eq!(result, 120);  // 5! = 120
}

// ============================================================================
// Protocol Chaining in Complex Expressions
// ============================================================================

/// Long protocol operation chains
#[test]
fn complex_protocol_chains() {
    let result: i64 = rune! {
        fn complex_calculation() -> i64 {
            // Chain multiple arithmetic protocols
            (((1 + 2) * 3) - 4) / 5 + ((6 + 7) * 8) - 9
        }

        complex_calculation()
    };
    // Calculate: (3 * 3 - 4) / 5 + (13 * 8) - 9 = (9 - 4) / 5 + 104 - 9 = 1 + 95 = 96
    assert_eq!(result, 96);
}

// ============================================================================
// Mixed Protocol Operations with Different Types
// ============================================================================

/// Operations involving multiple number types
#[test]
fn mixed_number_operations() {
    let result: f64 = rune! {
        fn mixed_calc() -> f64 {
            let x = 10;      // i64
            let y = 3.14;    // f64
            let z = 5;       // i64

            // Mix protocols with type conversion
            (x as f64) * y + (z as f64) / 2.0
        }

        mixed_calc()
    };
    assert!((result - 33.9).abs() < 0.0001);  // (10.0 * 3.14) + (5.0 / 2.0) ≈ 33.9
}

// ============================================================================
// Protocol Operations with Arrays and Indexing
// ============================================================================

/// Array operations with type checking
#[test]
fn array_protocol_operations() {
    let result: i64 = rune! {
        fn process_array() -> i64 {
            let arr = [1, 2, 3, 4, 5];

            // Access elements (INDEX_GET protocol)
            let sum = arr[0] + arr[1] + arr[2] + arr[3] + arr[4];
            // Get length (LEN protocol)
            let len = arr.len() as i64;
            sum / len
        }

        process_array()
    };
    assert_eq!(result, 3);  // (1+2+3+4+5)/5 = 15/5 = 3
}

// ============================================================================
// Protocol Operations with String Building
// ============================================================================

/// String manipulation protocols
#[test]
fn string_building_protocols() {
    let result: String = rune! {
        fn build_greeting(name: String) -> String {
            let greeting = "Hello";
            let punctuation = "!";

            // String concatenation protocol
            greeting + " " + name + punctuation
        }

        build_greeting("World")
    };
    assert_eq!(result, "Hello World!");
}

// ============================================================================
// Protocol Operations with Boolean Logic
// ============================================================================

/// Complex boolean operations
#[test]
fn complex_boolean_logic() {
    let result: bool = rune! {
        fn check_conditions(a: i64, b: i64, c: i64) -> bool {
            // Multiple comparison protocols
            (a > 0 && b < 100) || (c == 42 && b > a)
        }

        check_conditions(10, 50, 42)
    };
    // (10 > 0 && 50 < 100) || (42 == 42 && 50 > 10) = true || true = true
    assert!(result);
}

// ============================================================================
// Protocol Operations in Loop Contexts
// ============================================================================

/// Protocols within loops
#[test]
fn protocols_in_loops() {
    let result: i64 = rune! {
        fn sum_until_limit(limit: i64) -> i64 {
            let sum = 0;
            let i = 1;

            while i <= limit {
                sum = sum + i;
                i = i + 1;
            }

            sum
        }

        sum_until_limit(5)
    };
    // 1 + 2 + 3 + 4 + 5 = 15
    assert_eq!(result, 15);
}

// ============================================================================
// Protocol Operations with Option Handling
// ============================================================================

/// Option handling protocols (using None as fallback)
#[test]
fn option_protocol_handling() {
    let result: Value = rune! {
        fn safe_divide(a: i64, b: i64) {
            if b != 0 {
                a / b
            } else {
                0  // Fallback value
            }
        }

        safe_divide(10, 2)
    };
    // Just verify execution succeeds
    drop(result);
}

// ============================================================================
// Protocol Operations with Enum-like Patterns
// ============================================================================

/// Enum-like type with protocol operations
#[test]
fn enum_protocol_operations() {
    let result: i64 = rune! {
        fn calculate(kind, a, b) -> i64 {
            match kind {
                0 => a + b,        // Add
                1 => a * b,        // Multiply
                2 => a - b,        // Subtract
                _ => a / b,        // Divide
            }
        }

        // Test all operations
        let sum = calculate(0, 10, 5);
        let product = calculate(1, 4, 3);
        let difference = calculate(2, 10, 3);
        let quotient = calculate(3, 20, 4);

        sum + product + difference + quotient
    };
    assert_eq!(result, 39);  // 15 + 12 + 7 + 5 = 39
}

// ============================================================================
// Protocol Performance Scenarios
// ============================================================================

/// Performance: Many small operations
#[test]
fn performance_many_small_ops() {
    let result: i64 = rune! {
        fn many_ops() -> i64 {
            let total = 0;
            let i = 0;

            while i < 100 {
                total = total + i;
                i = i + 1;
            }

            total
        }

        many_ops()
    };
    // Sum of 0 to 99 = 99 * 100 / 2 = 4950
    assert_eq!(result, 4950);
}

/// Performance: Protocol chains
#[test]
fn performance_protocol_chains() {
    let result: f64 = rune! {
        fn chain_protocols() -> f64 {
            let x = 2.0;

            // Chain many float protocols
            let y = x * x;
            let z = y * y;
            let w = z * x;
            let v = w * y;
            let u = v * z;
            let result = u * x;
            result
        }

        chain_protocols()
    };
    // The actual calculation: 2 * 2 = 4; 4 * 4 = 16; 16 * 2 = 32; 32 * 4 = 128; 128 * 16 = 2048; 2048 * 2 = 4096.0
    assert_eq!(result, 4096.0);
}