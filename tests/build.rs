use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::Path;

fn discover_tests() -> io::Result<()> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR");
    let out_dir = env::var_os("OUT_DIR").expect("missing OUT_DIR");

    let mut f = fs::File::create(Path::new(&out_dir).join("tests.rs"))?;

    let tests = Path::new(&manifest_dir).join("tests");

    for entry in fs::read_dir(tests)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        if let Some(stem) = path.file_stem() {
            let path = path.canonicalize()?;

            writeln!(f, "#[path = {:?}]", path.display())?;
            writeln!(f, "mod {};", stem.to_string_lossy())?;
        }
    }

    Ok(())
}

fn main() {
    discover_tests().expect("failed to discover tests");
}
