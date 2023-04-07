prelude!();

#[test]
pub fn test_bug_428() {
    let (a, b): (String, String) = rune! {
        pub fn main() {
            let a = format!("{:>} = {:>}", "AB", 0x1234);
            let b = format!("{:>} = {:08}", "AB", 0x1234);
            (a, b)
        }
    };

    assert_eq!(a, "AB = 4660");
    assert_eq!(b, "AB = 00004660");
}
