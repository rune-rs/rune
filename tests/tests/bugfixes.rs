/// This was due to a binding optimization introduced in
/// 195b67821e5e3dc9f4f5371a7799d2fd08b43ce7, causing the local binding `x` to
/// simply alias the location of `dx`.
#[test]
fn test_pattern_binding_bug() {
    let out = rune! {
        i64 =>
        fn foo(dx) {
            let x = dx;

            for n in 0..10 {
                x = (x + dx);
            }

            x
        }

        pub fn main() {
            foo(3)
        }
    };

    assert_eq!(out, 3 * 10);
}
