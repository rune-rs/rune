//! Tests for `std::any::type_name_of_val(v)` for native types

prelude!();

use rune::alloc;

#[derive(Any, Debug)]
#[rune(item = ::native_crate)]
pub struct NativeStructWithProtocol;

impl NativeStructWithProtocol {
    #[rune::function(protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{self:?}")
    }
}

#[derive(Any)]
#[rune(item = ::native_crate)]
pub struct NativeStructWithoutProtocol;

fn make_native_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("native_crate")?;
    module.ty::<NativeStructWithProtocol>()?;
    module.function_meta(NativeStructWithProtocol::debug_fmt)?;
    module.ty::<NativeStructWithoutProtocol>()?;
    Ok(module)
}

#[test]
fn test_with_debug_fmt() {
    let t1 = NativeStructWithProtocol;

    let m = make_native_module().unwrap();

    let s: String = rune_n! {
        mod m,
        (t1,),
        pub fn main(v) { format!("{v:?}") }
    };

    assert_eq!(s, "NativeStructWithProtocol");
}

#[test]
fn test_without_debug_fmt() {
    let t1 = NativeStructWithoutProtocol;

    let m = make_native_module().unwrap();

    let result: String = rune_n! {
        mod m,
        (t1,),
        pub fn main(v) { format!("{v:?}") }
    };

    assert!(
        result.starts_with("<::native_crate::NativeStructWithoutProtocol object at 0x"),
        "Expected '<::native_crate::NativeStructWithoutProtocol object at 0x', got: {result:?}",
    );
}

/// What a failed `parse` prints.
///
/// The three parse errors were registered as types and nothing else, so they
/// fell back to the form above: `{:?}` on one wrote a heap address, which is
/// different on every run and says nothing about why the text did not parse.
#[test]
fn a_parse_error_says_what_went_wrong() {
    for (source, display, debug) in [
        (
            r#"if let Err(e) = i64::parse("x") { (format!("{e}"), format!("{e:?}")) } else { ("", "") }"#,
            "invalid digit found in string",
            "ParseIntError",
        ),
        (
            r#"if let Err(e) = u64::parse("-1") { (format!("{e}"), format!("{e:?}")) } else { ("", "") }"#,
            "invalid digit found in string",
            "ParseIntError",
        ),
        (
            r#"if let Err(e) = f64::parse("x") { (format!("{e}"), format!("{e:?}")) } else { ("", "") }"#,
            "invalid float literal",
            "ParseFloatError",
        ),
        (
            r#"if let Err(e) = "ab".parse::<char>() { (format!("{e}"), format!("{e:?}")) } else { ("", "") }"#,
            "too many characters in string",
            "ParseCharError",
        ),
    ] {
        let (actual_display, actual_debug): (String, String) = eval(source);

        assert_eq!(actual_display, display, "{source}");
        assert!(actual_debug.starts_with(debug), "{source}: {actual_debug}");
        assert!(!actual_debug.contains("0x"), "{source}: {actual_debug}");
    }
}
