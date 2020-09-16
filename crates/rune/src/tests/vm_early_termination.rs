macro_rules! test_case {
    ($kind:literal, $field:tt, $index:tt, $extra:literal) => {
        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ let m = {kind}; m[return true]; false }} {extra}"#,
                kind = $kind, extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ let m = {kind}; m[return true] = 0; false }} {extra}"#,
                kind = $kind, extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ let m = {kind}; m[{index}] = return true; false }} {extra}"#,
                kind = $kind, index = stringify!($index), extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ let m = {kind}; m.{field} = return true; false }} {extra}"#,
                kind = $kind, field = stringify!($field), extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ {kind}[return true]; false }} {extra}"#,
                kind = $kind, extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ {kind}[return true] = 0; false }} {extra}"#,
                kind = $kind, extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ {kind}[{index}] = return true; false }} {extra}"#,
                kind = $kind, index = stringify!($index), extra = $extra,
            )),
            true,
        };

        assert_eq! {
            rune!(bool => &format!(
                r#"fn main() {{ {kind}.{field} = return true; false }} {extra}"#,
                kind = $kind, field = stringify!($field), extra = $extra,
            )),
            true,
        };
    };
}

#[test]
fn test_object_like_early_term() {
    test_case!("#{}", test, "test", "");
}

#[test]
fn test_tuple_like_early_term() {
    test_case!("()", 0, 0, "");
}

#[test]
fn test_typed_object_early_term() {
    test_case!("Foo()", 0, 0, "struct Foo();");
}

#[test]
fn test_typed_tuple_early_term() {
    test_case!("Foo{test: 0}", test, "test", "struct Foo{test};");
}
