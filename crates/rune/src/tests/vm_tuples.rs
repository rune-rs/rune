prelude!();

#[test]
fn test_mutate_tuples() {
    let out: String = rune_s! { r#"
        pub fn main() {
            let m = ("Now", "You", "See", "Me");
            m.2 = "Don't";
            m.3 = "!";
            `${m.0} ${m.1} ${m.2} ${m.3}`
        }
    "# };
    assert_eq!(out, "Now You Don't !");
}
