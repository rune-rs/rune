use super::Fragment;

#[test]
fn test_fragment() {
    let fragment = Fragment::parse("abc*def.*");
    assert!(fragment.is_match("abc_xyz_def.exe"));
    assert!(fragment.is_match("abc_xyz_def."));
    assert!(!fragment.is_match("ab_xyz_def.exe"));
    assert!(!fragment.is_match("abcdef"));
    assert!(!fragment.is_match("abc_xyz_def"));

    let fragment = Fragment::parse("*def");
    assert!(fragment.is_match("abcdef"));
    assert!(!fragment.is_match("abcdeftrailing"));

    let fragment = Fragment::parse("abc*");
    assert!(fragment.is_match("abcdef"));
    assert!(!fragment.is_match("leadingabc"));
}
