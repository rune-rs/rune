use rune_tests::*;

#[test]
fn test_basic_assign() {
    assert_eq! {
        42,
        rune! { i64 =>
            pub fn main() { let a = 0; a = 42; a }
        }
    };
}

#[test]
fn test_assign_anon_object() {
    assert_eq! {
        42,
        rune! { i64 =>
            pub fn main() { let a = #{}; a.foo = #{}; a.foo.bar = 42; a.foo.bar }
        }
    };
}

#[test]
fn test_assign_anon_tuple() {
    assert_eq! {
        42,
        rune! { i64 =>
            pub fn main() { let a = ((0,),); (a.0).0 = 42; (a.0).0 }
        }
    };
}

#[test]
fn test_assign_struct() {
    assert_eq! {
        42,
        rune! { i64 =>
            struct Bar { padding, baz };
            struct Foo { bar, padding };

            pub fn main() {
                let foo = Foo { bar: (), padding: () };
                foo.bar = Bar { padding: (), baz: () };
                foo.bar.baz = 42;
                foo.bar.baz
            }
        }
    };
}

#[test]
fn test_assign_tuple() {
    assert_eq! {
        42,
        rune! { i64 =>
            struct Bar(baz, padding);
            struct Foo(padding, bar);

            pub fn main() {
                let foo = Foo((), ());
                foo.1 = Bar((), ());
                (foo.1).0 = 42;
                (foo.1).0
            }
        }
    };
}

#[test]
fn test_assign_assign_exprs() {
    assert_eq! {
        (4, (), ()),
        rune_s! {
            (i64, (), ()) => r#"
            pub fn main() {
                let a = #{b: #{c: #{d: 1}}};
                let b = 2;
                let c = 3;

                c = b = a.b.c = 4;
                (a.b.c, b, c)
            }
            "#
        }
    };
}
