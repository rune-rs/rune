use std::string::String;

use super::unescape::{parse_hex_escape, parse_unicode_escape};

macro_rules! input {
    ($string:expr) => {
        &mut String::from($string).char_indices().peekable()
    };
}

#[test]
fn test_parse_hex_escape() {
    assert!(parse_hex_escape(input!("a")).is_err());

    let c = parse_hex_escape(input!("7f")).unwrap();
    assert_eq!(c, 0x7f);
}

#[test]
fn test_parse_unicode_escape() {
    parse_unicode_escape(input!("{0}")).unwrap();

    let c = parse_unicode_escape(input!("{1F4AF}")).unwrap();
    assert_eq!(c, '💯');

    let c = parse_unicode_escape(input!("{1f4af}")).unwrap();
    assert_eq!(c, '💯');
}
