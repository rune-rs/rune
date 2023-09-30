use crate::error::Error;
use crate::vec::Vec;

#[test]
fn test_vec_macro() -> Result<(), Error> {
    let vec: Vec<u32> = try_vec![1, 2, 3];
    assert_eq!(vec, [1, 2, 3]);

    let vec: Vec<u32> = try_vec![1; 3];
    assert_eq!(vec, [1, 1, 1]);

    let vec: Vec<u32> = try_vec![];
    assert_eq!(vec, []);
    Ok(())
}
