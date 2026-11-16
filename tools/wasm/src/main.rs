//! Builds `rune-wasm` and bundles it with wasm-bindgen into `site/static/js`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use wasm_bindgen_cli_support::Bindgen;

const GLUE: &str = include_str!("../rune.js");

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("site/static/js");

    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let status = Command::new(cargo)
        .current_dir(&root)
        .args(["build", "-p", "rune-wasm", "--release", "--lib"])
        .args(["--target", "wasm32-unknown-unknown"])
        .status()
        .context("running cargo")?;

    if !status.success() {
        bail!("building rune-wasm failed: {status}");
    }

    let target_dir = match env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => root.join("target"),
    };

    let input = target_dir.join("wasm32-unknown-unknown/release/rune_wasm.wasm");

    if out.exists() {
        fs::remove_dir_all(&out)?;
    }

    fs::create_dir_all(&out)?;

    Bindgen::new()
        .input_path(&input)
        .web(true)?
        .out_name("rune_wasm")
        .generate(&out)
        .context("running wasm-bindgen")?;

    fs::write(out.join("rune.js"), GLUE)?;
    println!("Wrote {}", out.display());
    Ok(())
}
