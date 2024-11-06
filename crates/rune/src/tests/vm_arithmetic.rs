prelude!();

use VmErrorKind::*;

macro_rules! op_tests {
    ($ty:ty, ! $lhs:literal = $out:expr) => {
        op_tests!(@unary $ty, !, $lhs, $out);
    };

    ($ty:ty, $lhs:literal $op:tt $rhs:literal = $out:expr) => {
        op_tests!(@binary $ty, $lhs, $op, $rhs, $out);
    };

    (@binary $ty:ty, $lhs:literal, $op:tt, $rhs:literal, $out:expr) => {
        let out: $ty = rune!(let a = $lhs; let b = $rhs; a $op b);
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = {lhs}; let b = {rhs}; a {op}= b; a"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"struct Foo {{ padding, field }}; let a = Foo{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"enum Enum {{ Foo {{ padding, field }} }}; let a = Enum::Foo {{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = #{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b; a.field"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = (0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"struct Foo(padding, a); let a = Foo(0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"enum Enum {{ Foo(padding, a) }}; let a = Enum::Foo(0, {lhs}); let b = {rhs}; a.1 {op}= b; a.1"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = Ok({lhs}); let b = {rhs}; a.0 {op}= b; a.0"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = Some({lhs}); let b = {rhs}; a.0 {op}= b; a.0"#,
            lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);
    };

    (@unary $ty:ty, $op:tt, $lhs:literal, $out:expr) => {
        let out: $ty = rune!(let a = $lhs; $op a);
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = {lhs}; {op} a"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"struct Foo {{ padding, field }}; let a = Foo{{ padding: 0, field: {lhs} }}; {op} a.field"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"enum Enum {{ Foo {{ padding, field }} }}; let a = Enum::Foo {{ padding: 0, field: {lhs} }}; {op} a.field"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = #{{ padding: 0, field: {lhs} }}; {op} a.field"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = (0, {lhs}); {op} a.1"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"struct Foo(padding, a); let a = Foo(0, {lhs}); {op} a.1"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"enum Enum {{ Foo(padding, a) }}; let a = Enum::Foo(0, {lhs}); {op} a.1"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = Ok({lhs}); {op} a.0"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);

        let out: $ty = eval(&format!(
            r#"let a = Some({lhs}); {op} a.0"#,
            lhs = stringify!($lhs), op = stringify!($op),
        ));
        assert_eq!(out, $out);
    };
}

macro_rules! error_test {
    ($lhs:literal $op:tt $rhs:literal = $error:ident) => {
        assert_vm_error!(
            &format!(
                r#"let a = {lhs}; let b = {rhs}; a {op} b;"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"let a = {lhs}; let b = {rhs}; a {op}= b;"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"let a = #{{ padding: 0, field: {lhs} }}; let b = {rhs}; a.field {op}= b;"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );

        assert_vm_error!(
            &format!(
                r#"let a = (0, {lhs}); let b = {rhs}; a.1 {op}= b;"#,
                lhs = stringify!($lhs), rhs = stringify!($rhs), op = stringify!($op),
            ),
            $error => {}
        );
    }
}

#[test]
fn i64() {
    op_tests!(i64, 10 + 2 = 12);
    op_tests!(i64, 10 - 2 = 8);
    op_tests!(i64, 10 * 2 = 20);
    op_tests!(i64, 10 / 2 = 5);
    op_tests!(i64, 10 % 3 = 1);
    op_tests!(i64, 0b1100 & 0b0110 = 0b1100 & 0b0110);
    op_tests!(i64, 0b1100 ^ 0b0110 = 0b1100 ^ 0b0110);
    op_tests!(i64, 0b1100 | 0b0110 = 0b1100 | 0b0110);
    op_tests!(i64, 0b1100 << 2 = 0b1100 << 2);
    op_tests!(i64, 0b1100 >> 2 = 0b1100 >> 2);
    op_tests!(i64, !0b10100i64 = !0b10100i64);

    error_test!(9223372036854775807i64 + 2 = Overflow);
    error_test!(-9223372036854775808i64 - 2 = Underflow);
    error_test!(9223372036854775807i64 * 2 = Overflow);
    error_test!(10 / 0 = DivideByZero);
    error_test!(10 % 0 = DivideByZero);
    error_test!(0b1 << 64 = Overflow);
    error_test!(0b1 >> 64 = Underflow);
}

#[test]
fn u64() {
    op_tests!(u64, 0b1100 & 0b0110 = 0b1100 & 0b0110);
    op_tests!(u64, 0b1100 ^ 0b0110 = 0b1100 ^ 0b0110);
    op_tests!(u64, 0b1100 | 0b0110 = 0b1100 | 0b0110);
    op_tests!(u64, 0b1100 << 2 = 0b1100 << 2);
    op_tests!(u64, 0b1100 >> 2 = 0b1100 >> 2);
    op_tests!(u64, !0b10100u64 = !0b10100u64);

    error_test!(0b1 << 64 = Overflow);
    error_test!(0b1 >> 64 = Underflow);
}

#[test]
fn u8() {
    op_tests!(u8, 0b1100u8 & 0b0110u8 = 0b1100u8 & 0b0110u8);
    op_tests!(u8, 0b1100u8 ^ 0b0110u8 = 0b1100u8 ^ 0b0110u8);
    op_tests!(u8, 0b1100u8 | 0b0110u8 = 0b1100u8 | 0b0110u8);
    op_tests!(u8, 0b1100u8 << 2 = 0b1100u8 << 2);
    op_tests!(u8, 0b1100u8 >> 2 = 0b1100u8 >> 2);
    op_tests!(u64, !0b10100u8 = !0b10100u64);
}

#[test]
fn i8() {
    op_tests!(i8, 0b1100i8 & 0b0110i8 = 0b1100i8 & 0b0110i8);
    op_tests!(i8, 0b1100i8 ^ 0b0110i8 = 0b1100i8 ^ 0b0110i8);
    op_tests!(i8, 0b1100i8 | 0b0110i8 = 0b1100i8 | 0b0110i8);
    op_tests!(i8, 0b1100i8 << 2 = 0b1100i8 << 2);
    op_tests!(i8, 0b1100i8 >> 2 = 0b1100i8 >> 2);
    op_tests!(i64, !0b10100i8 = !0b10100i64);
}
