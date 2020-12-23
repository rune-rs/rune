//! Tests for `std::any::type_name_of_val(v)` for rune types

use rune_tests::*;

#[test]
fn test_trivial_types() {
    assert_eq!(
        rune! { Vec<String> =>
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
        },
        [
            "::std::bool".to_owned(),
            "::std::int".to_owned(),
            "::std::float".to_owned(),
            "::std::char".to_owned(),
            "::std::string::String".to_owned(),
            "::std::option::Option".to_owned()
        ]
    )
}

#[test]
fn test_fn_types() {
    assert_eq!(
        rune! { Vec<String> =>
            use std::any::type_name_of_val;
            fn foo() {}
            mod bar { pub fn foo() {} }
            pub fn main() {
                [type_name_of_val(foo), type_name_of_val(bar::foo)]
            }
        },
        vec!["foo", "bar::foo"]
    )
}

#[test]
fn test_struct() {
    assert_eq!(
        rune! { Vec<String> =>
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
        },
        vec!["X".to_owned(), "X::ctor".to_owned(), "X::foo".to_owned()]
    )
}

#[test]
fn test_enum() {
    assert_eq!(
        rune! { Vec<String> =>

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
        },
        vec!["E".to_owned(), "E".to_owned(), "E".to_owned()]
    )
}
