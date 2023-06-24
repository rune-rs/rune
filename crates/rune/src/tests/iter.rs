//! Test for iterator functions

prelude!();

#[test]
fn test_sum() {
    let out: u32 = rune!(
        pub fn main() {
            [1, 2, 3].iter().sum::<i64>()
        }
    );
    assert_eq!(out, 6)
}

#[test]
fn test_sum_negative() {
    let out: i32 = rune!(
        pub fn main() {
            [1, -2, 3].iter().sum::<i64>()
        }
    );
    assert_eq!(out, 2)
}

#[test]
fn test_prod() {
    let out: u32 = rune!(
        pub fn main() {
            [1, 2, 3, 6].iter().product::<i64>()
        }
    );
    assert_eq!(out, 36)
}

#[test]
fn test_prod_negative() {
    let out: i32 = rune!(
        pub fn main() {
            [-1, 2, 3, 6].iter().product::<i64>()
        }
    );
    assert_eq!(out, -36)
}

#[test]
fn test_prod_float() {
    let out: f32 = rune!(
        pub fn main() {
            [1.0, 0.5, 2.0, 3.0].iter().product::<f64>()
        }
    );
    assert_eq!(out, 3.0)
}

#[test]
fn test_prod_float_negative() {
    let out: f32 = rune!(
        pub fn main() {
            [1.0, 0.5, 2.0, 0.0 - 3.0].iter().product::<f64>()
        }
    );
    assert_eq!(out, -3.0)
}
