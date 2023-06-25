use crate::no_std::prelude::*;

/// Encode `string` as part of a quoted javascript string.
pub(crate) fn encode_quoted(out: &mut String, string: &str) {
    for c in string.chars() {
        let s = match c {
            '\\' => "\\\\",
            '\"' => "\\\"",
            c => {
                out.push(c);
                continue;
            }
        };

        out.push_str(s);
    }
}
