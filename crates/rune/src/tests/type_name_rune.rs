//! Tests for `std::any::type_name_of_val(v)` for rune types

prelude!();

#[test]
fn test_trivial_types() {
    let out: Vec<String> = rune! {
        use std::any::type_name_of_val;
        pub fn main() {
            [
                type_name_of_val(true),
                type_name_of_val(1),
                type_name_of_val(1.0),
                type_name_of_val('c'),
                type_name_of_val("s"),
                type_name_of_val(Some("s")),
            ]
        }
    };
    assert_eq!(
        out,
        [
            "::std::bool".to_owned(),
            "::std::i64".to_owned(),
            "::std::f64".to_owned(),
            "::std::char".to_owned(),
            "::std::string::String".to_owned(),
            "::std::option::Option".to_owned()
        ]
    )
}

#[test]
fn test_fn_types() {
    let out: Vec<String> = rune! {
        use std::any::type_name_of_val;
        fn foo() {}
        mod bar { pub fn foo() {} }
        pub fn main() {
            [type_name_of_val(foo), type_name_of_val(bar::foo)]
        }
    };
    assert_eq!(out, vec!["::std::ops::Function", "::std::ops::Function"]);
}

#[test]
fn test_struct() {
    let out: Vec<String> = rune! {
        use std::any::type_name_of_val;

        struct X{}
        impl X{
            fn foo(self) {}
            fn ctor() {
            }
        }
        pub fn main() {
            let x = X{};
            [type_name_of_val(x), type_name_of_val(X::ctor), type_name_of_val(X::foo)]
        }
    };

    assert_eq!(
        out,
        vec![
            "X".to_owned(),
            "::std::ops::Function".to_owned(),
            "::std::ops::Function".to_owned()
        ]
    )
}

#[test]
fn test_enum() {
    let out: Vec<String> = rune! {
        use std::any::type_name_of_val;

        enum E {
            A{ f },
            B(g),
            C,
        }

        pub fn main() {
            let ea = E::A { f: 1 };
            let eb = E::B(2);
            let ec = E::C;

            [type_name_of_val(ea), type_name_of_val(eb), type_name_of_val(ec)]
        }
    };
    assert_eq!(out, vec!["E".to_owned(), "E".to_owned(), "E".to_owned()]);
}
