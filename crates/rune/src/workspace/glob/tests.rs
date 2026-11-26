use super::{Fragment, Glob};
use crate::support::Result;

#[test]
fn test_fragment() -> Result<()> {
    let fragment = Fragment::parse("abc*def.*")?;
    assert!(fragment.is_match("abc_xyz_def.exe")?);
    assert!(fragment.is_match("abc_xyz_def.")?);
    assert!(!fragment.is_match("ab_xyz_def.exe")?);
    assert!(!fragment.is_match("abcdef")?);
    assert!(!fragment.is_match("abc_xyz_def")?);

    let fragment = Fragment::parse("*def")?;
    assert!(fragment.is_match("abcdef")?);
    assert!(!fragment.is_match("abcdeftrailing")?);

    let fragment = Fragment::parse("abc*")?;
    assert!(fragment.is_match("abcdef")?);
    assert!(!fragment.is_match("leadingabc")?);
    Ok(())
}

/// A `**` walks every directory beneath it. Without this the walk stopped at
/// the first file it met, so anything queued after that file was never seen.
#[test]
#[cfg(not(miri))]
fn double_star_walks_past_files() -> Result<()> {
    let root = std::env::temp_dir().join(format!("rune-glob-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("a/b/c"))?;
    std::fs::write(root.join("a/file.txt"), "")?;
    std::fs::write(root.join("a/b/c/found.d.rn"), "")?;

    let glob = Glob::new(&root, "**/*.d.rn")?;
    let mut found = std::vec::Vec::new();

    for path in glob.matcher()? {
        found.push(path?);
    }

    std::fs::remove_dir_all(&root)?;
    assert_eq!(found, [root.join("a/b/c/found.d.rn")]);
    Ok(())
}
