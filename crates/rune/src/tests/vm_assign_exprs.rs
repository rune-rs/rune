prelude!();

#[test]
fn test_basic_assign() {
    let out: i64 = rune! {
        pub fn main() { let a = 0; a = 42; a }
    };

    assert_eq!(out, 42);
}

#[test]
fn test_assign_underscore() {
    let out: i64 = rune! {
        pub fn main() { let _a = 0; _a = 42; _a }
    };

    assert_eq!(out, 42);
}

#[test]
fn test_assign_underscores() {
    let out: i64 = rune! {
        pub fn main() { let ___ = 0; ___ = 42; ___ }
    };

    assert_eq!(out, 42);
}

#[test]
fn test_assign_anon_object() {
    let out: i64 = rune! {
        pub fn main() { let a = #{}; a.foo = #{}; a.foo.bar = 42; a.foo.bar }
    };

    assert_eq!(out, 42);
}

#[test]
fn test_assign_anon_tuple() {
    let out: i64 = rune! {
        pub fn main() { let a = ((0,),); (a.0).0 = 42; (a.0).0 }
    };

    assert_eq!(out, 42);
}

#[test]
fn test_assign_struct() {
    let out: i64 = rune! {
        struct Bar { padding, baz };
        struct Foo { bar, padding };

        pub fn main() {
            let foo = Foo { bar: (), padding: () };
            foo.bar = Bar { padding: (), baz: () };
            foo.bar.baz = 42;
            foo.bar.baz
        }
    };
    assert_eq!(out, 42);
}

#[test]
fn test_assign_tuple() {
    let out: i64 = rune! {
        struct Bar(baz, padding);
        struct Foo(padding, bar);

        pub fn main() {
            let foo = Foo((), ());
            foo.1 = Bar((), ());
            (foo.1).0 = 42;
            (foo.1).0
        }
    };
    assert_eq!(out, 42);
}

#[test]
fn test_assign_assign_exprs() {
    let out: (i64, (), ()) = rune_s! {
        r#"
        pub fn main() {
            let a = #{b: #{c: #{d: 1}}};
            let b = 2;
            let c = 3;

            c = b = a.b.c = 4;
            (a.b.c, b, c)
        }
        "#
    };
    assert_eq!(out, (4, (), ()));
}
