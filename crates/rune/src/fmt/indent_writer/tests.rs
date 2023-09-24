use crate::alloc::fmt::TryWrite;
use crate::support::Result;

use super::IndentedWriter;

#[test]
fn test_roundtrip() -> Result<()> {
    let mut writer = IndentedWriter::new()?;
    writer.try_write_str("hello\nworld\n")?;
    assert_eq!(
        writer.into_inner(),
        [&b"hello"[..], &b"world"[..], &b""[..]]
    );
    Ok(())
}

#[test]
fn test_roundtrip_with_indent() -> Result<()> {
    let mut writer = IndentedWriter::new()?;
    writer.indent();
    writer.try_write_str("hello\nworld\n")?;
    assert_eq!(
        writer.into_inner(),
        [&b"    hello"[..], &b"    world"[..], &b""[..]]
    );
    Ok(())
}
