//! Hashing a value of a type which was declared in a script.

prelude!();

/// A type declared in a script can be a key.
///
/// Equality on one is structural - the same type, the same variant, and fields
/// which are equal in turn - and `Value::hash_with` had no case for it at all,
/// so it fell through to a protocol nothing implements. Two values which
/// compare equal could not be looked up by one another, which is to say the
/// type could be compared but not used.
#[test]
fn a_script_declared_type_can_be_a_key() {
    let out: i64 = eval(
        r#"
        use std::collections::HashMap;

        struct S { a, b }

        let m = HashMap::new();
        m.insert(S { a: 1, b: "x" }, 10);
        m.insert(S { a: 2, b: "y" }, 20);

        let hit = m.get(S { a: 1, b: "x" }).unwrap();
        let miss = m.get(S { a: 9, b: "z" }).is_none();

        hit + if miss { 1 } else { 0 } + m.len()
        "#,
    );

    assert_eq!(out, 13);
}

/// A variant is hashed as the variant it is.
///
/// Two variants of the same enum carrying the same fields are not equal, so
/// they must not land on one another.
#[test]
fn a_variant_is_hashed_as_the_variant_it_is() {
    let out: i64 = eval(
        r#"
        use std::collections::HashSet;

        enum E { A, B(x), C(x) }

        let s = HashSet::new();
        s.insert(E::A);
        s.insert(E::B(1));
        s.insert(E::C(1));
        s.insert(E::B(1));

        s.len()
        "#,
    );

    assert_eq!(out, 3);
}

/// Hashing is structural all the way down.
#[test]
fn a_field_is_hashed_by_what_it_holds() {
    let out: bool = eval(
        r#"
        use std::collections::HashMap;

        struct Inner { v }
        struct Outer { inner, list }

        let m = HashMap::new();
        m.insert(Outer { inner: Inner { v: 1 }, list: [1, 2] }, "yes");

        m.get(Outer { inner: Inner { v: 1 }, list: [1, 2] }) == Some("yes")
            && m.get(Outer { inner: Inner { v: 2 }, list: [1, 2] }) is Option
            && m.get(Outer { inner: Inner { v: 2 }, list: [1, 2] }).is_none()
        "#,
    );

    assert!(out);
}

/// What hashes together compares equal.
///
/// Values of two different types are not equal - comparing them is an error -
/// so nothing is owed about their hashes, but two of the same type which
/// compare equal have to hash the same or a map built from them loses entries.
#[test]
fn what_compares_equal_hashes_the_same() {
    let out: bool = eval(
        r#"
        use std::collections::HashMap;

        struct S { a }

        let m = HashMap::new();

        for i in 0..64 {
            m.insert(S { a: i }, i);
        }

        let ok = true;

        for i in 0..64 {
            if m.get(S { a: i }) != Some(i) {
                ok = false;
            }
        }

        ok && m.len() == 64
        "#,
    );

    assert!(out);
}
