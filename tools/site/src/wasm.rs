use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use wasm_bindgen_cli_support::Bindgen;

use crate::digest;

const GLUE: &str = r#"import init from "./rune_wasm.js";

await init({ module_or_path: new URL("./rune_wasm_bg.wasm", import.meta.url) });
"#;

/// Build `rune-wasm` and bindgen it into a directory under `out/js` named for
/// what is in it, which is returned.
///
/// GitHub Pages lets browsers and its own caches keep a file for some minutes
/// without asking again, so a page from one deploy could load the script of
/// another, and the script one wasm module and the bindings of another. A new
/// build is a new address instead.
pub fn build(root: &Path, out: &Path) -> Result<String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let status = Command::new(cargo)
        .current_dir(root)
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
    let js = out.join("js");

    if js.exists() {
        fs::remove_dir_all(&js)?;
    }

    let staging = js.join("staging");
    fs::create_dir_all(&staging)?;

    Bindgen::new()
        .input_path(&input)
        .web(true)?
        .out_name("rune_wasm")
        .generate(&staging)
        .context("running wasm-bindgen")?;

    fs::write(staging.join("rune.js"), GLUE)?;

    let name = digest::of_tree(&staging)?;
    fs::rename(&staging, js.join(&name))?;
    Ok(name)
}

/// The directory an earlier [`build`] left in `out/js`.
pub fn current(out: &Path) -> Result<String> {
    let js = out.join("js");
    let mut names = Vec::new();

    for entry in fs::read_dir(&js)
        .with_context(|| format!("{}: build without --no-wasm first", js.display()))?
    {
        let entry = entry?;

        if entry.path().join("rune.js").is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    match <[String; 1]>::try_from(names) {
        Ok([name]) => Ok(name),
        Err(names) => bail!(
            "{}: expected one build, found {}; build without --no-wasm",
            js.display(),
            names.len()
        ),
    }
}
