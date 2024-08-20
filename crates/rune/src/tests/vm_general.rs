#![allow(clippy::unit_cmp)]

prelude!();

#[test]
fn test_template_string() {
    let out: String = eval(
        r#"
        pub fn main() {
            let name = "John Doe";
            `Hello ${name}, I am ${1 - 10} years old!`
        }
    "#,
    );
    assert_eq!(out, "Hello John Doe, I am -9 years old!");

    // Contrived expression with an arbitrary sub-expression.
    // This tests that the temporary variables used during calculations do not
    // accidentally clobber the scope.
    let out: String = eval(
        r#"
        pub fn main() {
            let name = "John Doe";

            `Hello ${name}, I am ${{
                let a = 20;
                a += 2;
                a
            }} years old!`
        }
    "#,
    );
    assert_eq!(out, "Hello John Doe, I am 22 years old!");
}

#[test]
fn test_complex_field_access() {
    let out: Option<i64> = eval(
        r#"
        fn foo() {
            #{hello: #{world: 42}}
        }

        pub fn main() {
            Some((foo()).hello["world"])
        }
    "#,
    );
    assert_eq!(out, Some(42));
}
