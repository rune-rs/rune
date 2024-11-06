prelude!();

#[test]
fn test_wildcard_precedence() {
    rune_assert! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use {a::*, b::*};
        use b::Foo;
        Foo is b::Foo
    };

    rune_assert! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use {b::*, a::*};
        use a::Foo;
        Foo is a::Foo
    };

    rune_assert! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use a::*;
        use b::*;
        use a::Foo;
        Foo is a::Foo
    };

    rune_assert! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use a::Foo;
        use a::*;
        use b::*;
        Foo is a::Foo
    };
}
