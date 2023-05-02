use super::*;

#[test]
fn test_roundtrip() {
    let mut writer = IndentedWriter::new();
    writer.write_all(b"hello\nworld\n").unwrap();
    assert_eq!(writer.into_inner(), vec!["hello", "world", ""]);
}

#[test]
fn test_roundtrip_with_indent() {
    let mut writer = IndentedWriter::new();
    writer.indent();
    writer.write_all(b"hello\nworld\n").unwrap();
    assert_eq!(writer.into_inner(), vec!["    hello", "    world", ""]);
}
