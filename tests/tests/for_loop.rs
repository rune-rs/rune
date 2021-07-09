use rune_tests::*;

#[test]
fn test_binding_pattern() {
    let out = rune! { i64 =>
        pub fn main() {
            let data = [(1, 2), (2, 3), (3, 4)];
            let out = 0;

            for (a, b) in data {
                out += a * b;
            }

            out
        }
    };

    assert_eq!(out, 2 + 2 * 3 + 3 * 4);
}

#[test]
fn test_simple_binding() {
    let out = rune! { i64 =>
        pub fn main() {
            let data = [1, 2, 3, 4];
            let out = 0;

            for v in data {
                out += v;
            }

            out
        }
    };

    assert_eq!(out, 1 + 2 + 3 + 4);
}

#[test]
fn test_ignore_binding() {
    let out = rune! { i64 =>
        pub fn main() {
            let data = [1, 2, 3, 4];
            let out = 0;

            for _ in data {
                out += 1;
            }

            out
        }
    };

    assert_eq!(out, 4);
}
