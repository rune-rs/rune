//! A small utility to build the guts of https://rune-rs.github.io

use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use std::env;
use std::io;
use std::path::Path;
use std::process::Command;
use tar::Archive;

fn main() -> Result<()> {
    let url = match env::var("ZOLA_URL") {
        Ok(url) => url,
        Err(..) => bail!("missing ZOLA_URL"),
    };

    let target = Path::new("target");
    let bin = target.join("zola");

    if !bin.is_file() {
        println!("Downloading: {}", url);
        let bytes = reqwest::blocking::get(&url)?.bytes()?;
        let decoder = GzDecoder::new(io::Cursor::new(bytes.as_ref()));
        let mut archive = Archive::new(decoder);
        archive.unpack(target)?;
    }

    if !bin.is_file() {
        bail!("Missing bin: {}", bin.display());
    }

    let mut it = env::args();
    it.next();

    let status = Command::new(bin).args(it).status()?;
    std::process::exit(status.code().unwrap());
}
