use super::Names;
use crate::support::Result;

#[test]
fn insert() -> Result<()> {
    let mut names = Names::default();
    assert!(!names.contains(["test"])?);
    assert!(!names.insert(["test"]).unwrap());
    assert!(names.contains(["test"])?);
    assert!(names.insert(["test"]).unwrap());
    Ok(())
}

#[test]
fn contains() -> Result<()> {
    let mut names = Names::default();
    assert!(!names.contains(["test"])?);
    assert!(!names.insert(["test"]).unwrap());
    assert!(names.contains(["test"])?);
    assert!(names.insert(["test"]).unwrap());
    Ok(())
}
