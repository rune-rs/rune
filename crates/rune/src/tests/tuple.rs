prelude!();

fn make_module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module
        .function("receive_tuple", |(_, _): (Value, Value)| ())
        .build()?;
    module
        .function(
            "receive_vec_tuple",
            |VecTuple((_, _)): VecTuple<(Value, Value)>| (),
        )
        .build()?;
    Ok(module)
}

/// This ensures that as values are being unpacked from a tuple, that neither it
/// nor its arguments are taken over by the receiving function.
#[test]
fn test_tuple_ownership() {
    let m = make_module().expect("Failed to make module");

    rune_n! {
        &m,
        (),
        () => pub fn main() {
            let a = [];
            let b = [];
            let tuple = (a, b);
            receive_tuple(tuple);
            assert!(is_readable(tuple));
            assert!(is_readable(a));
            assert!(is_readable(b));
        }
    };

    rune_n! {
        &m,
        (),
        () => pub fn main() {
            let a = [];
            let b = [];
            let vec = [a, b];
            receive_vec_tuple(vec);
            assert!(is_readable(vec));
            assert!(is_readable(a));
            assert!(is_readable(b));
        }
    };
}
