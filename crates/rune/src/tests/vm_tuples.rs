#[test]
fn test_mutate_tuples() {
    assert_eq! {
        rune! {
            String => r#"
            fn main() {
                let m = ("Now", "You", "See", "Me");
                m.2 = "Don't";
                m.3 = "!";
                `{m.0} {m.1} {m.2} {m.3}`
            }
            "#
        },
        "Now You Don't !",
    };
}
