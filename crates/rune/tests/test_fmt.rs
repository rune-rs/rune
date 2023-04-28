use rune::fmt::layout;

#[test]
fn test_layout() {
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

    assert_eq!(layout(input.to_owned()).unwrap(), expected);
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

    assert_eq!(layout(input.to_owned()).unwrap(), expected);
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

    assert_eq!(layout(input.to_owned()).unwrap(), expected);
}
