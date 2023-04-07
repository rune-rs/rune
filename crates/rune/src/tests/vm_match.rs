prelude!();

#[test]
fn test_match_primitives() {
    let out: bool = rune! {
        pub fn main() { match false { false => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match b'a' { b'a' => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match 'a' { 'a' => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match "hello world" { "hello world" => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match b"hello world" { b"hello world" => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match 42 { 42 => true, _ => false } }
    };
    assert!(out);

    let out: bool = rune! {
        pub fn main() { match -42 { -42 => true, _ => false } }
    };
    assert!(out);
}

#[test]
fn test_path_type_match() {
    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() {
            match Custom::A { Custom::A => true, _ => false }
        }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() {
            match Custom::B(0) { Custom::A => true, _ => false }
        }
    };
    assert_eq!(out, false);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() {
            match Custom::B(0) { Custom::B(0) => true, _ => false }
        }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B { a } }
        pub fn main() {
            match (Custom::B { a: 0 }) { Custom::B { a: 0 } => true, _ => false }
        }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B { a } }
        fn test(a) { a == 0 }

        pub fn main() {
            match (Custom::B { a: 0 }) { Custom::B { a } if test(a) => true, _ => false }
        }
    };
    assert_eq!(out, true);
}

#[test]
fn test_struct_matching() {
    let out: i64 = rune! {
        struct Foo { a, b }

        pub fn main() {
            let foo = Foo {
                a: 1,
                b: 2,
            };

            match foo {
                Foo { a, b } => a + b,
                _ => 0,
            }
        }
    };
    assert_eq!(out, 3);

    let out: i64 = rune! {
        struct Foo { a, b }

        pub fn main() {
            let b = 2;

            let foo = Foo {
                a: 1,
                b,
            };

            match foo {
                Foo { a, b } => a + b,
                _ => 0,
            }
        }
    };
    assert_eq!(out, 3);
}
