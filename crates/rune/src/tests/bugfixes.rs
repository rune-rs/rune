prelude!();

/// This was due to a binding optimization introduced in
/// 195b67821e5e3dc9f4f5371a7799d2fd08b43ce7, causing the local binding `x` to
/// simply alias the location of `dx`.
#[test]
fn test_pattern_binding_bug() {
    let out: i64 = rune! {
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

    assert_eq!(out, 3 * 11);
}

/// Bug where string patterns just do not work when part of a match statement
/// inside of an inst fn call.
#[test]
fn test_string_pattern_in_instance_fn_bug() {
    rune! {
        enum Inst { A, Unknown }

        pub fn works() {
            let program = [];
            let inst = match "a" { "a" => Inst::A, _ => Inst::Unknown };
            program.push(inst);
            assert_eq!(program, [Inst::A]);
        }

        pub fn broken() {
            let program = [];
            program.push(match "a" { "a" => Inst::A, _ => Inst::Unknown });
            assert_eq!(program, [Inst::A]);
        }

        pub fn main() {
            works();
            broken();
        }
    };
}
