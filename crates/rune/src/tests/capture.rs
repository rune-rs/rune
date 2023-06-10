prelude!();

#[test]
fn test_closure() {
    let number: i64 = rune! {
        pub async fn main() {
            let a = 1;
            let b = 2;
            let closure = { let c = 4; |d, e| |f| a + b + c + d + e + f };
            closure(8, 16)(32)
        }
    };

    assert_eq!(number, 1 + 2 + 4 + 8 + 16 + 32);
}

#[test]
fn test_async() {
    let number: i64 = rune! {
        pub async fn main() {
            let a = 1;
            let b = 2;
            let closure = async { let c = 4; |d, e| |f| a + b + c + d + e + f };
            closure.await(8, 16)(32)
        }
    };

    assert_eq!(number, 1 + 2 + 4 + 8 + 16 + 32);
}
