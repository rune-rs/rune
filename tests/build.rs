use std::env;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::Path;

fn main() -> io::Result<()> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing CARGO_MANIFEST_DIR"))?;
    let out_dir = env::var_os("OUT_DIR")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing OUT_DIR"))?;

    let mut f = fs::File::create(Path::new(&out_dir).join("tests.rs"))?;

    let tests = Path::new(&manifest_dir).join("tests");

    for entry in fs::read_dir(tests)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(stem) = path.file_stem() {
            let path = path.canonicalize()?;

            writeln!(f, "#[path = {:?}]", path.display())?;
            writeln!(f, "mod {};", stem.to_string_lossy())?;
        }
    }

    Ok(())
}
