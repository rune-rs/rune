//! Test for iterator functions

#[test]
fn test_sum() {
    assert_eq!(rune!(u32 => pub fn main() { [1, 2, 3].iter().sum() }), 6)
}

#[test]
fn test_sum_negative() {
    assert_eq!(rune!(i32 => pub fn main() { [1, -2, 3].iter().sum() }), 2)
}

#[test]
fn test_prod() {
    assert_eq!(
        rune!(u32 => pub fn main() { [1, 2, 3, 6].iter().product() }),
        36
    )
}

#[test]
fn test_prod_negative() {
    assert_eq!(
        rune!(i32 => pub fn main() { [-1, 2, 3, 6].iter().product() }),
        -36
    )
}

#[test]
fn test_prod_float() {
    assert_eq!(
        rune!(f32 => pub fn main() { [1.0, 0.5, 2.0, 3.0].iter().product() }),
        3.0
    )
}

#[test]
fn test_prod_float_negative() {
    assert_eq!(
        rune!(f32 => pub fn main() { [1.0, 0.5, 2.0, 0.0 - 3.0].iter().product() }),
        -3.0
    )
}
