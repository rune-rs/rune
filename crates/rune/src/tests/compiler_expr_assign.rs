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

/// The target of a compound assignment is assembled before the right-hand side
/// and named by the instruction which is written after it, so it has to stay
/// allocated across both.
///
/// It was freed as soon as it had been named, which let the right-hand side be
/// given the same address - so `o.a.b += 1` stored `1` over the object it was
/// meant to be modifying and then went looking for a field `b` on it.
#[test]
fn assign_binop_does_not_clobber_its_target() {
    let value: i64 = eval("let o = #{a: #{b: 1}}; o.a.b += 41; o.a.b");
    assert_eq!(value, 42);

    // The right-hand side being something which needs assembling of its own is
    // what makes it want an address.
    let value: i64 = eval("fn n() { 5 } let o = #{a: #{b: 1}}; o.a.b += n(); o.a.b");
    assert_eq!(value, 6);

    let value: i64 = eval("let o = #{a: #{b: 1}}; let x = 2; o.a.b += x * 3 + 1; o.a.b");
    assert_eq!(value, 8);

    // A tuple field of a chain is the same shape with the other instruction.
    let value: i64 = eval("let o = #{a: (1, 2)}; o.a.0 += 9; o.a.0");
    assert_eq!(value, 10);

    // Deeper, and through an index, which reach the same step.
    let value: i64 = eval("let o = #{a: #{b: #{c: 1}}}; o.a.b.c += 1; o.a.b.c");
    assert_eq!(value, 2);

    let value: i64 = eval("let v = [#{b: 1}]; v[0].b += 1; v[0].b");
    assert_eq!(value, 2);

    // What it modifies is shared with whatever else points at it.
    let value: Vec<i64> = eval("let i = #{b: 1}; let o = #{a: i}; o.a.b += 1; [o.a.b, i.b]");
    assert_eq!(value, [2, 2]);

    // The shapes which were already right are still right.
    let value: i64 = eval("let o = #{b: 1}; o.b += 1; o.b");
    assert_eq!(value, 2);
}

/// Every operator against every shape which can be assigned to in place.
///
/// The assembler handles each target shape separately - a variable is an
/// address, a field is an address plus a slot, an index is a read and a write
/// around the operation - so a bug in one of them says nothing about the
/// others, and one of them was quietly writing over its own target.
#[test]
fn assign_binop_over_every_target_and_operator() {
    // Each entry sets up a target holding `12`, applies `<op>= 5` to it, and
    // reads it back.
    let shapes = [
        ("let t = 12; t {op}= 5; t", "variable"),
        ("let o = #{v: 12}; o.v {op}= 5; o.v", "field"),
        (
            "let o = #{a: #{v: 12}}; o.a.v {op}= 5; o.a.v",
            "nested field",
        ),
        (
            "let o = #{a: #{b: #{v: 12}}}; o.a.b.v {op}= 5; o.a.b.v",
            "twice nested field",
        ),
        ("let t = (12, 0); t.0 {op}= 5; t.0", "tuple field"),
        (
            "let o = #{a: (12, 0)}; o.a.0 {op}= 5; o.a.0",
            "nested tuple field",
        ),
        ("let v = [12]; v[0] {op}= 5; v[0]", "index"),
        ("let v = [[12]]; v[0][0] {op}= 5; v[0][0]", "nested index"),
        ("let o = #{v: 12}; o[\"v\"] {op}= 5; o.v", "object index"),
        (
            "use std::collections::HashMap; \
             let m = HashMap::new(); m.insert(\"v\", 12); m[\"v\"] {op}= 5; m[\"v\"]",
            "map index",
        ),
        (
            "let o = #{a: [12]}; o.a[0] {op}= 5; o.a[0]",
            "field then index",
        ),
        (
            "let v = [#{v: 12}]; v[0].v {op}= 5; v[0].v",
            "index then field",
        ),
    ];

    let operators = [
        ("+", 17),
        ("-", 7),
        ("*", 60),
        ("/", 2),
        ("%", 2),
        ("&", 4),
        ("|", 13),
        ("^", 9),
        ("<<", 384),
        (">>", 0),
    ];

    for (template, what) in shapes {
        for (op, expected) in operators {
            let source = template.replace("{op}", op);
            let value: i64 = eval(&source);

            assert_eq!(value, expected, "{what} with `{op}=`: {source}");
        }
    }
}
