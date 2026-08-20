prelude!();

use ErrorKind::*;

#[test]
fn assign_expr() {
    assert_parse!(r#"let var = 1; var = 42;"#);

    assert_errors! {
        r#"1 = 42;"#,
        span!(0, 6), UnsupportedAssignExpr
    };
}

#[test]
fn mut_let() {
    assert_errors! {
        r#"let mut var = 1;"#,
        span!(4, 7), UnsupportedMut
    };
}

/// An index is not an address, so there is nothing for an assigning operator to
/// be applied to in place. What is at the index is read out, the operator is
/// applied to that, and the result is put back.
///
/// This used to be reported as "Unsupported binary expression", which said
/// nothing about what was unsupported, and the way around it -
/// `v[i] = v[i] + 1` - evaluates the index twice.
#[test]
fn assign_binop_through_an_index() {
    let value: Vec<i64> = eval("let v = [1, 2]; v[0] += 10; v[1] -= 1; v");
    assert_eq!(value, [11, 1]);

    let value: i64 = eval("let v = [3]; v[0] *= 4; v[0] /= 2; v[0] <<= 2; v[0]");
    assert_eq!(value, 24);

    // Anything which can be indexed, not just a vector.
    let value: i64 = eval("let o = #{a: 1}; o[\"a\"] += 5; o.a");
    assert_eq!(value, 6);

    let value: i64 = eval(
        "use std::collections::HashMap; \
         let m = HashMap::new(); m.insert(\"k\", 1); m[\"k\"] += 2; m[\"k\"]",
    );
    assert_eq!(value, 3);

    // The index is an expression of its own, and nesting works.
    let value: Vec<Vec<i64>> = eval("let v = [[1]]; v[0][0] += 9; v");
    assert_eq!(value, [[10]]);

    // The value of the assignment is the unit, as it is everywhere else.
    let value: bool = eval("let v = [1]; let a = (v[0] += 1); a is Tuple && v[0] == 2");
    assert!(value);
}

/// The target and the index are assembled once and used by both ends of the
/// operation, so anything they do happens once.
#[test]
fn assign_binop_through_an_index_evaluates_the_index_once() {
    let value: Vec<i64> = eval(
        "let calls = [0]; \
         fn f(calls) { calls[0] = calls[0] + 1; 0 } \
         let v = [1]; \
         v[f(calls)] += 5; \
         [v[0], calls[0]]",
    );

    assert_eq!(value, [6, 1]);
}
