prelude!();

#[test]
fn while_loop() {
    let _: () = rune! {
        pub fn main() {
            let n = 0;

            while n < 10 {
                n += 1;
            }

            assert_eq!(n, 10);
        }
    };
}

#[test]
fn for_loop() {
    let _: () = rune! {
        pub fn main() {
            let n = 0;

            for i in 0..10 {
                n += i;
            }

            assert_eq!(n, 45);
        }
    };
}
