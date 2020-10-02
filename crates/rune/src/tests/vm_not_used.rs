#[test]
fn test_not_used() {
    assert_eq! {
        rune_s! {
            () => r#"
            fn main() {
                0;
                4.1;
                'a';
                b'a';
                "Hello World";
                b"Hello World";
                [1, 2, 3];
                (1, 2, 3, 4);
                #{"foo": 42, "bar": [1, 2, 3, 4]};
            }
            "#
        },
        (),
    };
}
