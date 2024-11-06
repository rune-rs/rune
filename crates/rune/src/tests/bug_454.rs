prelude!();

/// See https://github.com/rune-rs/rune/issues/454
#[test]
pub fn test_bug_454() {
    let _: () = rune! {
        struct Test;

        fn call(a, b) {
            a + b
        }

        impl Test {
            fn call(self) {
                call(1, 2)
            }
        }

        let test = Test;
        assert_eq!(test.call(), 3);
    };
}
