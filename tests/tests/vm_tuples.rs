use rune_tests::*;

#[test]
fn test_mutate_tuples() {
    assert_eq! {
        rune_s! { String => r#"
            pub fn main() {
                let m = ("Now", "You", "See", "Me");
                m.2 = "Don't";
                m.3 = "!";
                `${m.0} ${m.1} ${m.2} ${m.3}`
            }
        "#},
        "Now You Don't !",
    };
}
