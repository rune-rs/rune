#[test]
fn test_wildcard_precedence() {
    assert!(rune! { bool =>
        mod a { struct Foo; }
        mod b { struct Foo; }
        use a::*;
        use b::*;
        pub fn main() { Foo is b::Foo }
    });

    assert!(rune! { bool =>
        mod a { struct Foo; }
        mod b { struct Foo; }
        use {a::*, b::*};
        pub fn main() { Foo is b::Foo }
    });

    assert!(rune! { bool =>
        mod a { struct Foo; }
        mod b { struct Foo; }
        use {b::*, a::*};
        pub fn main() { Foo is a::Foo }
    });

    assert!(rune! { bool =>
        mod a { struct Foo; }
        mod b { struct Foo; }
        use a::*;
        use b::*;
        use a::Foo;
        pub fn main() { Foo is a::Foo }
    });

    assert!(rune! { bool =>
        mod a { struct Foo; }
        mod b { struct Foo; }
        use a::Foo;
        use a::*;
        use b::*;
        pub fn main() { Foo is a::Foo }
    });
}
