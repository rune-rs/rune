use anyhow::{anyhow, Context as _};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| anyhow!("missing OUT_DIR"))?);

    let version = if let Ok(rune_version) = env::var("RUNE_VERSION") {
        rune_version
    } else {
        let output = Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output()?;

        let rev = std::str::from_utf8(&output.stdout)?.trim();
        format!("git-{}", rev)
    };

    fs::write(out_dir.join("version.txt"), &version).context("writing version.txt")?;
    Ok(())
}
