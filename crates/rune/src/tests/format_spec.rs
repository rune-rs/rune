//! Tests for the specification a value is formatted under.
//!
//! A specification which is accepted has to be honoured: one which is parsed
//! and then ignored formats the value as if it had not been written at all,
//! which is worse than rejecting it.

prelude!();

/// The width applies to every scalar which is written directly rather than
/// through a protocol.
///
/// Unsigned integers and booleans were written without any of it, so the width
/// they were given was dropped while the one a signed integer was given was
/// not.
#[test]
fn width_applies_to_unsigned_and_boolean() {
    let out: String = eval(r#"format!("{:5}", 42u64)"#);
    assert_eq!(out, "42   ");

    let out: String = eval(r#"format!("{:>5}", 42u64)"#);
    assert_eq!(out, "   42");

    let out: String = eval(r#"format!("{:^5}", 42u64)"#);
    assert_eq!(out, " 42  ");

    let out: String = eval(r#"format!("{:05}", 42u64)"#);
    assert_eq!(out, "00042");

    let out: String = eval(r#"format!("{:+}", 42u64)"#);
    assert_eq!(out, "+42");

    let out: String = eval(r#"format!("{:5}", true)"#);
    assert_eq!(out, "true ");

    let out: String = eval(r#"format!("{:>7}", false)"#);
    assert_eq!(out, "  false");

    let out: String = eval(r#"format!("{:*^7}", true)"#);
    assert_eq!(out, "*true**");

    // The same applies to how they are written for debugging.
    let out: String = eval(r#"format!("{:>5?}", 42u64)"#);
    assert_eq!(out, "   42");

    let out: String = eval(r#"format!("{:>7?}", true)"#);
    assert_eq!(out, "   true");

    // What a signed integer does is unchanged.
    let out: String = eval(r#"format!("{:5}", 42)"#);
    assert_eq!(out, "42   ");
}

/// The sign a number is written with is written whether or not it is being
/// zero padded, since the padding goes between the sign and the digits.
///
/// A sign asked for with `+` was dropped when zero padding was asked for as
/// well, so `{:+05}` wrote a number which looked unsigned.
#[test]
fn the_sign_survives_zero_padding() {
    let out: String = eval(r#"format!("{:+05}", 42)"#);
    assert_eq!(out, "+0042");

    let out: String = eval(r#"format!("{:+05}", -42)"#);
    assert_eq!(out, "-0042");

    let out: String = eval(r#"format!("{:05}", 42)"#);
    assert_eq!(out, "00042");

    let out: String = eval(r#"format!("{:+08.2}", 1.5)"#);
    assert_eq!(out, "+0001.50");

    let out: String = eval(r#"format!("{:+08.2}", -1.5)"#);
    assert_eq!(out, "-0001.50");

    let out: String = eval(r#"format!("{:+05}", 42u64)"#);
    assert_eq!(out, "+0042");

    // Without `+` nothing is written for a positive number.
    let out: String = eval(r#"format!("{:08.2}", 1.5)"#);
    assert_eq!(out, "00001.50");
}

/// A number written in a radix carries a prefix when one is asked for with
/// `#`, which goes between the sign and the digits.
///
/// The flag was parsed and then ignored, so `{:#x}` wrote the same thing `{:x}`
/// did.
#[test]
fn the_alternate_flag_writes_a_radix_prefix() {
    let out: String = eval(r#"format!("{:#x}", 255)"#);
    assert_eq!(out, "0xff");

    let out: String = eval(r#"format!("{:#X}", 255)"#);
    assert_eq!(out, "0xFF");

    let out: String = eval(r#"format!("{:#b}", 5)"#);
    assert_eq!(out, "0b101");

    // The prefix belongs to the digits, so the padding goes after it.
    let out: String = eval(r#"format!("{:#010x}", 255)"#);
    assert_eq!(out, "0x000000ff");

    // And it counts towards the width.
    let out: String = eval(r#"format!("{:#>8x}", 255)"#);
    assert_eq!(out, "######ff");

    let out: String = eval(r#"format!("{:>8}", 255)"#);
    assert_eq!(out, "     255");

    // Without the flag nothing is written in front.
    let out: String = eval(r#"format!("{:x}", 255)"#);
    assert_eq!(out, "ff");

    let out: String = eval(r#"format!("{:010x}", 255)"#);
    assert_eq!(out, "00000000ff");
}

/// The precision a value is written with may be zero, and it cuts a string
/// short.
///
/// It was kept as a non-zero number, so `{:.0}` was the same as writing no
/// precision at all, and it was only ever applied to floats.
#[test]
fn the_precision_may_be_zero_and_applies_to_strings() {
    let out: String = eval(r#"format!("{:.0}", 1.5)"#);
    assert_eq!(out, "2");

    let out: String = eval(r#"format!("{:.0}", 1.4)"#);
    assert_eq!(out, "1");

    let out: String = eval(r#"format!("{:.2}", 1.5)"#);
    assert_eq!(out, "1.50");

    let out: String = eval(r#"format!("{:.1}", "abc")"#);
    assert_eq!(out, "a");

    let out: String = eval(r#"format!("{:.0}", "abc")"#);
    assert_eq!(out, "");

    let out: String = eval(r#"format!("{:.5}", "abc")"#);
    assert_eq!(out, "abc");

    // The width still applies to what is left of the string.
    let out: String = eval(r#"format!("{:5.1}", "abc")"#);
    assert_eq!(out, "a    ");

    let out: String = eval(r#"format!("{:>5.2}", "abcdef")"#);
    assert_eq!(out, "   ab");

    // Cutting counts characters rather than bytes.
    let out: String = eval(r#"format!("{:.2}", "héllo")"#);
    assert_eq!(out, "hé");
}

/// An unsigned integer is written in a radix the same way a signed one is.
///
/// Only signed integers were accepted, so writing an unsigned one in hex or
/// binary was rejected as a value which cannot be formatted.
#[test]
fn a_radix_accepts_unsigned_integers() {
    let out: String = eval(r#"format!("{:x}", 255u64)"#);
    assert_eq!(out, "ff");

    let out: String = eval(r#"format!("{:X}", 255u64)"#);
    assert_eq!(out, "FF");

    let out: String = eval(r#"format!("{:b}", 5u64)"#);
    assert_eq!(out, "101");

    let out: String = eval(r#"format!("{:#010x}", 255u64)"#);
    assert_eq!(out, "0x000000ff");

    let out: String = eval(r#"format!("{:>6b}", 5u64)"#);
    assert_eq!(out, "   101");

    // The whole range is written rather than being cut short by the sign.
    let out: String = eval(r#"format!("{:x}", 18446744073709551615u64)"#);
    assert_eq!(out, "ffffffffffffffff");
}

/// The sign a number is written with belongs to its digits, so the padding
/// which surrounds it goes outside of it.
///
/// The sign was written before the padding no matter which alignment was asked
/// for, so `{:^+8}` wrote the sign hard against the left edge while it centred
/// the digits, and a sign a negative number carries ended up somewhere else
/// again because that one is written with the digits.
#[test]
fn the_sign_is_padded_along_with_the_digits() {
    let out: String = eval(r#"format!("{:^+9}", 5)"#);
    assert_eq!(out, "   +5    ");

    let out: String = eval(r#"format!("{:>+9}", 5)"#);
    assert_eq!(out, "       +5");

    let out: String = eval(r#"format!("{:*<+9}", 5)"#);
    assert_eq!(out, "+5*******");

    // Which is where a negative number puts the sign it carries.
    let out: String = eval(r#"format!("{:^9}", -5)"#);
    assert_eq!(out, "   -5    ");

    let out: String = eval(r#"format!("{:>9}", -5)"#);
    assert_eq!(out, "       -5");

    // Zero padding is the exception, since it is written between the sign and
    // the digits rather than around them.
    let out: String = eval(r#"format!("{:^+09}", 5)"#);
    assert_eq!(out, "+00000005");

    // The same goes for floats.
    let out: String = eval(r#"format!("{:^+9.1}", 1.5)"#);
    assert_eq!(out, "  +1.5   ");

    let out: String = eval(r#"format!("{:^9.1}", -1.5)"#);
    assert_eq!(out, "  -1.5   ");
}

/// A number written in a radix is written as the bits it is made of, so it has
/// no sign of its own and the smallest signed number is written like any other.
///
/// The magnitude was written with a sign in front of it instead, so `{:05x}`
/// wrote `-0005` where the bits say `fffffffffffffffb`, and negating the
/// smallest signed number to get at its magnitude overflowed.
#[test]
fn a_radix_writes_the_bits_of_a_negative_number() {
    let out: String = eval(r#"format!("{:x}", -5)"#);
    assert_eq!(out, "fffffffffffffffb");

    let out: String = eval(r#"format!("{:05x}", -5)"#);
    assert_eq!(out, "fffffffffffffffb");

    let out: String = eval(r#"format!("{:#010x}", -5)"#);
    assert_eq!(out, "0xfffffffffffffffb");

    let out: String = eval(r#"format!("{:b}", -5)"#);
    assert_eq!(
        out,
        "1111111111111111111111111111111111111111111111111111111111111011"
    );

    // A sign asked for with `+` is still written, since it is asked for rather
    // than being carried by the number.
    let out: String = eval(r#"format!("{:+x}", -5)"#);
    assert_eq!(out, "+fffffffffffffffb");

    // The smallest signed number has no magnitude to negate.
    let out: String = eval(r#"format!("{:x}", -9223372036854775808)"#);
    assert_eq!(out, "8000000000000000");

    let out: String = eval(r#"format!("{:025}", -9223372036854775808)"#);
    assert_eq!(out, "-000009223372036854775808");

    let out: String = eval(r#"format!("{}", -9223372036854775808)"#);
    assert_eq!(out, "-9223372036854775808");
}

/// The message of the first fatal diagnostic `source` raises.
#[track_caller]
fn compile_error(source: &str) -> String {
    let mut diagnostics = Diagnostics::new();

    if crate::tests::compile_helper(source, &mut diagnostics).is_ok() {
        panic!("Source should not compile:\n{source}");
    }

    for diagnostic in diagnostics.into_diagnostics() {
        if let diagnostics::Diagnostic::Fatal(fatal) = diagnostic {
            if let diagnostics::FatalDiagnosticKind::CompileError(error) = fatal.into_kind() {
                return error.to_string();
            }
        }
    }

    panic!("No compile error was raised for:\n{source}");
}

/// A width or a precision has to be small enough to be written.
///
/// Both were dropped when they did not fit, so `{:.4294967296}` formatted the
/// value as if no precision had been asked for at all, and one which fit in the
/// number it was parsed into but not in the one the formatter takes brought the
/// whole process down instead.
#[test]
fn a_width_or_precision_which_is_too_large_is_reported() {
    for source in [
        r#"format!("{:70000}", 1)"#,
        r#"format!("{:99999999999999999999}", 1)"#,
    ] {
        let error = compile_error(source);
        assert!(error.contains("width"), "{error}");
        assert!(error.contains("65535"), "{error}");
    }

    for source in [
        r#"format!("{:.70000}", 1.0)"#,
        r#"format!("{:.4294967296}", 1.0)"#,
        r#"format!("{:.99999999999999999999}", 1.0)"#,
        r#"format!("{:.*}", 70000, 1.0)"#,
    ] {
        let error = compile_error(source);
        assert!(error.contains("precision"), "{error}");
        assert!(error.contains("65535"), "{error}");
    }

    // The largest which can be asked for is still honoured.
    let out: String = eval(r#"format!("{:65535}", 1)"#);
    assert_eq!(out.len(), 65535);

    let out: String = eval(r#"format!("{:.65535}", 1.0)"#);
    assert_eq!(out.len(), 65537);
}
