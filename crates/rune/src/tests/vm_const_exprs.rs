macro_rules! test_op {
    ($ty:ty => $lhs:literal $op:tt $rhs:literal = $result:literal) => {
        assert_eq!(
            $result,
            rune!($ty => &format!(
                r#"const A = {lhs}; const B = {rhs}; const VALUE = A {op} B; fn main() {{ VALUE }}"#,
                lhs = $lhs, rhs = $rhs, op = stringify!($op),
            ))
        );
    }
}

#[test]
fn test_const_values() {
    assert_eq!(
        true,
        rune!(bool => r#"const VALUE = true; fn main() { VALUE }"#)
    );
    assert_eq!(
        "Hello World",
        rune!(String => r#"const VALUE = "Hello World"; fn main() { VALUE }"#)
    );
}

#[test]
fn test_integer_ops() {
    test_op!(i64 => 1 + 2 = 3);
    test_op!(i64 => 2 - 1 = 1);
    test_op!(i64 => 8 / 2 = 4);
    test_op!(i64 => 8 * 2 = 16);
    test_op!(i64 => 0b1010 << 2 = 0b101000);
    test_op!(i64 => 0b1010 >> 2 = 0b10);
    test_op!(bool => 1 < 2 = true);
    test_op!(bool => 2 < 2 = false);
    test_op!(bool => 1 <= 1 = true);
    test_op!(bool => 2 <= 1 = false);
    test_op!(bool => 3 > 2 = true);
    test_op!(bool => 2 > 2 = false);
    test_op!(bool => 1 >= 1 = true);
    test_op!(bool => 0 >= 2 = false);
}
