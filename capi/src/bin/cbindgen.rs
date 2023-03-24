use std::env;
use std::error::Error;
use std::path::PathBuf;

use cbindgen::Config;

fn main() -> Result<(), Box<dyn Error>> {
    let crate_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing CARGO_MANIFEST_DIR")?);
    let config = crate_dir.join("cbindgen.toml");
    let header = crate_dir.join("rune.h");

    let config = Config::from_file(config)?;

    cbindgen::Builder::new()
        .with_config(config)
        .with_crate(crate_dir)
        .generate()?
        .write_to_file(header);

    Ok(())
}
