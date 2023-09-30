use crate::alloc::prelude::*;
use crate::alloc::{String, Vec};
use crate::fmt::FormattingError;
use crate::support::Result;

pub(crate) fn layout_string(contents: String) -> Result<Vec<u8>, FormattingError> {
    super::layout_source(&contents)
}

#[test]
fn test_layout_string() -> Result<()> {
    let input = r#"
        fn main() {
            let x = 1; let y = 2;           x + y
        }
        "#;

    let expected = r#"fn main() {
    let x = 1;
    let y = 2;
    x + y
}
"#;

    assert_eq!(layout_string(input.try_to_owned()?)?, expected.as_bytes());

    Ok(())
}

#[test]
fn test_layout_two_fns() -> Result<()> {
    let input = r#"
        fn main() {
            let x = 1; let y = 2;           x + y
        }

        fn foo() {
            let x = 1; let y = 2;           x + y
        }
        "#;

    let expected = r#"fn main() {
    let x = 1;
    let y = 2;
    x + y
}

fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
"#;

    assert_eq!(layout_string(input.try_to_owned()?)?, expected.as_bytes());

    Ok(())
}

#[test]
fn test_layout_two_fns_with_comments() -> Result<()> {
    let input = r#"
        fn main() {
            let x = 1; let y = 2;           x + y
        }

        /// foo
        fn foo() {
            let x = 1; let y = 2;           x + y
        }
        "#;

    let expected = r#"fn main() {
    let x = 1;
    let y = 2;
    x + y
}

/// foo
fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
"#;

    assert_eq!(layout_string(input.try_to_owned()?)?, expected.as_bytes());

    Ok(())
}

#[test]
fn test_macrocall_whitespace() -> Result<()> {
    let input = r#"
        fn main() {
            foo!();

            1 + 2
        }
        "#;

    let expected = r#"fn main() {
    foo!();

    1 + 2
}
"#;

    let output = layout_string(input.try_to_owned()?)?;
    let output = layout_string(String::from_utf8(output)?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    Ok(())
}

#[test]
fn test_macrocall_whitespace2() -> Result<()> {
    let input = r#"make_function!(root_fn => { "Hello World!" });
// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let expected = r#"make_function!(root_fn => { "Hello World!" });
// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let output = layout_string(input.try_to_owned()?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    let output = layout_string(String::from_utf8(output)?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    let output = layout_string(String::from_utf8(output)?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    Ok(())
}

#[test]
fn test_macrocall_whitespace3() -> Result<()> {
    let input = r#"make_function!(root_fn => { "Hello World!" });




// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let expected = r#"make_function!(root_fn => { "Hello World!" });




// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let output = layout_string(input.try_to_owned()?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    let output = layout_string(String::from_utf8(output)?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    let output = layout_string(String::from_utf8(output)?)?;
    assert_eq!(std::str::from_utf8(&output)?, expected);
    Ok(())
}
