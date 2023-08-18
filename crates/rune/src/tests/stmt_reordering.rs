prelude!();

#[test]
fn test_stmt_reordering() {
    let len: i64 = rune! {
        pub fn main() {
            let len = 0;
            let value = String::from("Hello");
            len = value.len();
            let value2 = drop(value);
            len
        }
    };

    assert_eq!(len, 5);
}

#[test]
fn test_const_stmt_reordering() {
    let n: i64 = rune! {
        const fn foo() {
            let n = 0;
            n = 1;
            let n = 2;
            n
        }

        pub fn main() {
            foo()
        }
    };

    assert_eq!(n, 2);
}
