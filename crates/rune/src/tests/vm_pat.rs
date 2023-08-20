prelude!();

#[test]
fn test_ignore_binding() {
    let out: bool = rune! {
        fn returns_unit(n) {
            let _ = 100;
        }

        pub fn main() {
            returns_unit(1) is Tuple
        }
    };
    assert_eq!(out, true);
}

#[test]
fn test_name_binding() {
    let out: bool = rune! {
        fn returns_unit(n) {
            let a = 100;
        }

        pub fn main() {
            returns_unit(1) is Tuple
        }
    };
    assert_eq!(out, true);
}

#[test]
fn test_match_binding() {
    let out: bool = rune! {
        fn returns_unit(n) {
            let [..] = [1, 2, 3];
        }

        pub fn main() {
            returns_unit(1) is Tuple
        }
    };
    assert_eq!(out, true);
}
