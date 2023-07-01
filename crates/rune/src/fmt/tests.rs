use crate::no_std::prelude::*;

use crate::fmt::FormattingError;

pub(crate) fn layout_string(contents: String) -> Result<Vec<u8>, FormattingError> {
    super::layout_source(&contents)
}

#[test]
fn test_layout_string() {
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

    assert_eq!(
        layout_string(input.to_owned()).unwrap(),
        expected.as_bytes()
    );
}

#[test]
fn test_layout_two_fns() {
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

    assert_eq!(
        layout_string(input.to_owned()).unwrap(),
        expected.as_bytes()
    );
}

#[test]
fn test_layout_two_fns_with_comments() {
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

    assert_eq!(
        layout_string(input.to_owned()).unwrap(),
        expected.as_bytes()
    );
}

#[test]
fn test_macrocall_whitespace() {
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

    let output = layout_string(input.to_owned()).unwrap();
    let output = layout_string(String::from_utf8(output).unwrap()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
}

#[test]
fn test_macrocall_whitespace2() {
    let input = r#"make_function!(root_fn => { "Hello World!" });
// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let expected = r#"make_function!(root_fn => { "Hello World!" });
// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let output = layout_string(input.to_owned()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    let output = layout_string(String::from_utf8(output).unwrap()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    let output = layout_string(String::from_utf8(output).unwrap()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
}

#[test]
fn test_macrocall_whitespace3() {
    let input = r#"make_function!(root_fn => { "Hello World!" });




// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let expected = r#"make_function!(root_fn => { "Hello World!" });




// NB: we put the import in the bottom to test that import resolution isn't order-dependent.
"#;

    let output = layout_string(input.to_owned()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    let output = layout_string(String::from_utf8(output).unwrap()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    let output = layout_string(String::from_utf8(output).unwrap()).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
}
