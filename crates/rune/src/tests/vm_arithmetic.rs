prelude!();

use VmErrorKind::*;

macro_rules! op_tests {
    ($ty:ty, $lhs:literal $op:tt $rhs:literal = $out:expr) => {
        let out: $ty = rune!(pub fn main() { let a = $lhs; let b = $rhs; a $op b});
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"pub fn main() {{ let a = {lhs}; let b = {rhs}; a {op}= b; a }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"struct Foo {{ padding, field }}; pub fn main() {{ let a = Foo{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"enum Enum {{ Foo {{ padding, field }} }}; pub fn main() {{ let a = Enum::Foo {{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"pub fn main() {{ let a = #{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"pub fn main() {{ let a = (0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1 }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"struct Foo(padding, a); pub fn main() {{ let a = Foo(0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1 }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"enum Enum {{ Foo(padding, a) }}; pub fn main() {{ let a = Enum::Foo(0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1 }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"pub fn main() {{ let a = Ok({lhs}); let b = {rhs}; a.0 {op}= b; a.0 }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = rune_s!(&format!(
            r#"pub fn main() {{ let a = Some({lhs}); let b = {rhs}; a.0 {op}= b; a.0 }}"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);
    }
}

macro_rules! error_test {
    ($lhs:literal $op:tt $rhs:literal = $error:ident) => {
        assert_vm_error!(
            &format!(
                r#"pub fn main() {{ let a = {lhs}; let b = {rhs}; a {op} b; }}"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"pub fn main() {{ let a = {lhs}; let b = {rhs}; a {op}= b; }}"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"pub fn main() {{ let a = #{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; }}"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"pub fn main() {{ let a = (0, {lhs}); let b = {rhs}; a.1 {op}= b; }}"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );
    }
}

#[test]
fn test_add() {
    op_tests!(i64, 10 + 2 = 12);
    error_test!(9223372036854775807i64 + 2 = Overflow);
}

#[test]
fn test_sub() {
    op_tests!(i64, 10 - 2 = 8);
    error_test!(-9223372036854775808i64 - 2 = Underflow);
}

#[test]
fn test_mul() {
    op_tests!(i64, 10 * 2 = 20);
    error_test!(9223372036854775807i64 * 2 = Overflow);
}

#[test]
fn test_div() {
    op_tests!(i64, 10 / 2 = 5);
    error_test!(10 / 0 = DivideByZero);
}

#[test]
fn test_rem() {
    op_tests!(i64, 10 % 3 = 1);
    error_test!(10 % 0 = DivideByZero);
}

#[test]
fn test_bit_ops_i64() {
    op_tests!(i64, 0b1100 & 0b0110 = 0b1100 & 0b0110);
    op_tests!(i64, 0b1100 ^ 0b0110 = 0b1100 ^ 0b0110);
    op_tests!(i64, 0b1100 | 0b0110 = 0b1100 | 0b0110);
    op_tests!(i64, 0b1100 << 2 = 0b1100 << 2);
    op_tests!(i64, 0b1100 >> 2 = 0b1100 >> 2);
    error_test!(0b1 << 64 = Overflow);
    error_test!(0b1 >> 64 = Underflow);
}

#[test]
fn test_bit_ops_u8() {
    op_tests!(u8, 0b1100u8 & 0b0110u8 = 0b1100u8 & 0b0110u8);
    op_tests!(u8, 0b1100u8 ^ 0b0110u8 = 0b1100u8 ^ 0b0110u8);
    op_tests!(u8, 0b1100u8 | 0b0110u8 = 0b1100u8 | 0b0110u8);
    op_tests!(u8, 0b1100u8 << 2 = 0b1100u8 << 2);
    op_tests!(u8, 0b1100u8 >> 2 = 0b1100u8 >> 2);
    error_test!(0b1u8 << 8 = Overflow);
    error_test!(0b1u8 >> 8 = Underflow);
}

#[test]
fn test_bitwise_not_i64() {
    let out: i64 = rune!(
        pub fn main() {
            let a = 0b10100;
            !a
        }
    );
    assert_eq!(out, !0b10100);
}

#[test]
fn test_bitwise_not_u8() {
    let out: u8 = rune!(
        pub fn main() {
            let a = 0b10100u8;
            !a
        }
    );
    assert_eq!(out, !0b10100u8);
}
