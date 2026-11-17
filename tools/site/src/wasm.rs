use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use wasm_bindgen_cli_support::Bindgen;

const GLUE: &str = r#"import init from "./rune_wasm.js";

await init({ module_or_path: new URL("./rune_wasm_bg.wasm", import.meta.url) });
"#;

/// Build `rune-wasm` and bindgen it into `out/js`.
pub fn build(root: &Path, out: &Path) -> Result<()> {
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

    fs::create_dir_all(&js)?;

    Bindgen::new()
        .input_path(&input)
        .web(true)?
        .out_name("rune_wasm")
        .generate(&js)
        .context("running wasm-bindgen")?;

    fs::write(js.join("rune.js"), GLUE)?;
    Ok(())
}
