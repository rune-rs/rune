//! A small utility to build the guts of https://rune-rs.github.io

use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use std::env;
use std::io;
use std::path::Path;
use std::process::Command;
use tar::Archive;

const URL: &str = env!("ZOLA_URL");

fn main() -> Result<()> {
    let target = Path::new("target");
    let bin = target.join("zola");

    if !bin.is_file() {
        println!("Downloading: {}", URL);
        let bytes = reqwest::blocking::get(URL)?.bytes()?;
        let mut archive = Archive::new(GzDecoder::new(io::Cursor::new(bytes.as_ref())));
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
