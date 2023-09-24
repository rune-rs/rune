use crate::alloc::{self, String};

/// Encode `string` as part of a quoted javascript string.
pub(crate) fn encode_quoted(out: &mut String, string: &str) -> alloc::Result<()> {
    for c in string.chars() {
        let s = match c {
            '\\' => "\\\\",
            '\"' => "\\\"",
            c => {
                out.try_push(c)?;
                continue;
            }
        };

        out.try_push_str(s)?;
    }

    Ok(())
}
