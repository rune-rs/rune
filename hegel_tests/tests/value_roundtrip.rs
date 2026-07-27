use std::collections::HashMap;

use hegel::generators::{self, Generator};
use rune::runtime::{from_value, to_value};

/// Property: converting a primitive Rust value into a Rune `Value` and back
/// yields the original value, over the full domain of each type.
#[hegel::test]
fn value_roundtrip_preserves_primitives(tc: hegel::TestCase) {
    let n: i64 = tc.draw(generators::integers::<i64>());
    let out: i64 = from_value(to_value(n).expect("to_value")).expect("from_value");
    assert_eq!(out, n);

    let n: u64 = tc.draw(generators::integers::<u64>());
    let out: u64 = from_value(to_value(n).expect("to_value")).expect("from_value");
    assert_eq!(out, n);

    let f: f64 = tc.draw(generators::floats::<f64>());
    let out: f64 = from_value(to_value(f).expect("to_value")).expect("from_value");
    assert_eq!(out.to_bits(), f.to_bits(), "value: {f:?}");

    let b: bool = tc.draw(generators::booleans());
    let out: bool = from_value(to_value(b).expect("to_value")).expect("from_value");
    assert_eq!(out, b);

    let c: char = tc.draw(
        generators::integers::<u32>()
            .max_value(0x10_FFFF)
            .map(|n| char::from_u32(n).unwrap_or('\u{FFFD}')),
    );
    let out: char = from_value(to_value(c).expect("to_value")).expect("from_value");
    assert_eq!(out, c);
}

/// Property: converting compound Rust values (strings, vectors, maps, options,
/// tuples) into a Rune `Value` and back yields the original.
#[hegel::test]
fn value_roundtrip_preserves_compound_values(tc: hegel::TestCase) {
    let s: String = tc.draw(generators::text());
    let out: String = from_value(to_value(s.clone()).expect("to_value")).expect("from_value");
    assert_eq!(out, s);

    let v: Vec<i64> = tc.draw(generators::vecs(generators::integers::<i64>()));
    let out: Vec<i64> = from_value(to_value(v.clone()).expect("to_value")).expect("from_value");
    assert_eq!(out, v);

    let m: HashMap<String, i64> = tc.draw(generators::hashmaps(
        generators::text().max_size(16),
        generators::integers::<i64>(),
    ));
    let out: HashMap<String, i64> =
        from_value(to_value(m.clone()).expect("to_value")).expect("from_value");
    assert_eq!(out, m);

    let o: Option<i64> = tc.draw(generators::optional(generators::integers::<i64>()));
    let out: Option<i64> = from_value(to_value(o).expect("to_value")).expect("from_value");
    assert_eq!(out, o);

    let t: (i64, String, bool) = tc.draw(generators::tuples!(
        generators::integers::<i64>(),
        generators::text().max_size(16),
        generators::booleans(),
    ));
    let out: (i64, String, bool) =
        from_value(to_value(t.clone()).expect("to_value")).expect("from_value");
    assert_eq!(out, t);
}
