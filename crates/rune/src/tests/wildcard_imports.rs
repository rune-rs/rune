prelude!();

#[test]
fn test_wildcard_precedence() {
    assert!(rune! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use {a::*, b::*};
        use b::Foo;
        pub fn main() { Foo is b::Foo }
    });

    assert!(rune! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use {b::*, a::*};
        use a::Foo;
        pub fn main() { Foo is a::Foo }
    });

    assert!(rune! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use a::*;
        use b::*;
        use a::Foo;
        pub fn main() { Foo is a::Foo }
    });

    assert!(rune! {
        mod a { pub struct Foo; }
        mod b { pub struct Foo; }
        use a::Foo;
        use a::*;
        use b::*;
        pub fn main() { Foo is a::Foo }
    });
}
