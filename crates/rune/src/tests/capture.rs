prelude!();

#[test]
fn test_closure() {
    let number: i64 = rune! {
        let a = 1;
        let b = 2;
        let closure = { let c = 4; |d, e| |f| a + b + c + d + e + f };
        closure(8, 16)(32)
    };

    assert_eq!(number, 1 + 2 + 4 + 8 + 16 + 32);
}

#[test]
fn test_async() {
    let number: i64 = rune! {
        let a = 1;
        let b = 2;
        let closure = async { let c = 4; |d, e| |f| a + b + c + d + e + f };
        closure.await(8, 16)(32)
    };

    assert_eq!(number, 1 + 2 + 4 + 8 + 16 + 32);
}

/// A block which yields one of its own locals has to produce it into an address
/// which outlives the block, or closing the block clears the address the caller
/// is about to read.
#[test]
fn test_block_value_from_local() {
    let number: i64 = rune! {
        fn main() {
            let f = || { let t = 1; t };
            f()
        }

        main()
    };

    assert_eq!(number, 1);

    let number: i64 = rune! {
        fn main() {
            { let t = 2; t }
        }

        main()
    };

    assert_eq!(number, 2);
}
