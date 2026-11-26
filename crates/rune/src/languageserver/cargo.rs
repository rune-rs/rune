//! Declaration sources bundled by cargo crates.
//!
//! A crate which embeds rune, or which provides modules to it, can describe
//! what it installs in `.d.rn` files and name them in its manifest:
//!
//! ```toml
//! [package.metadata.rune]
//! declarations = ["rune/*.d.rn"]
//! ```
//!
//! The language server asks `cargo metadata` about the cargo project it was
//! opened in and compiles what every package in it names beside each script,
//! so that what a host provides resolves in an editor.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use relative_path::RelativePath;

use crate::alloc::Vec;
use crate::workspace::glob::Glob;

/// The name of a cargo manifest.
const CARGO_MANIFEST: &str = "Cargo.toml";

/// How long `cargo metadata` may take before it is given up on. The server
/// does nothing else while it waits, so this is what bounds how long an editor
/// is left without answers.
const TIMEOUT: Duration = Duration::from_secs(10);

/// What produces the output of `cargo metadata` for the cargo project a
/// directory is in, or nothing if it is in none.
///
/// The server spawns cargo, while its tests hand it the output of one.
pub(crate) type Metadata =
    rust_alloc::boxed::Box<dyn FnMut(&Path) -> Result<Option<rust_alloc::vec::Vec<u8>>> + Send>;

/// Whether `path` names a cargo manifest, whose change is what makes the
/// declarations it names worth looking up again.
pub(super) fn is_manifest(path: &str) -> bool {
    path.ends_with(CARGO_MANIFEST)
        && path[..path.len() - CARGO_MANIFEST.len()]
            .chars()
            .next_back()
            .is_none_or(|c| c == '/' || c == '\\')
}

/// The default [`Metadata`], which runs `cargo metadata` for the nearest
/// `Cargo.toml` at or above `dir`.
pub(super) fn spawn(dir: &Path) -> Result<Option<rust_alloc::vec::Vec<u8>>> {
    let Some(manifest) = dir
        .ancestors()
        .map(|dir| dir.join(CARGO_MANIFEST))
        .find(|path| path.is_file())
    else {
        return Ok(None);
    };

    // Asking about the whole dependency graph is what lets a crate depended on
    // bring its declarations, but it needs that graph resolved, which is not
    // done offline in a fresh checkout. Asking about the workspace alone
    // still finds what its own packages bring.
    match run(&manifest, &[]) {
        Ok(output) => Ok(Some(output)),
        Err(_error) => {
            tracing::warn!("cargo metadata: {_error}; trying without dependencies");
            Ok(Some(run(&manifest, &["--no-deps"])?))
        }
    }
}

/// Run `cargo metadata` for `manifest` with `extra` arguments, bounded by
/// [`TIMEOUT`].
fn run(manifest: &Path, extra: &[&str]) -> Result<rust_alloc::vec::Vec<u8>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let mut child = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .args(extra)
        .arg("--manifest-path")
        .arg(manifest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning cargo")?;

    // The pipes are drained while waiting, since a child which fills one
    // waits for it to be read and would otherwise never exit.
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);

    let deadline = Instant::now() + TIMEOUT;

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        if Instant::now() >= deadline {
            _ = child.kill();
            _ = child.wait();
            bail!("timed out after {}s", TIMEOUT.as_secs());
        }

        thread::sleep(Duration::from_millis(10));
    };

    let stdout = join(stdout)?;
    let stderr = join(stderr)?;

    if !status.success() {
        let stderr = std::string::String::from_utf8_lossy(&stderr);
        bail!("{status}: {}", stderr.trim());
    }

    Ok(stdout)
}

type Drain = thread::JoinHandle<std::io::Result<rust_alloc::vec::Vec<u8>>>;

fn drain<R>(mut reader: R) -> Drain
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = rust_alloc::vec::Vec::new();
        reader.read_to_end(&mut output)?;
        Ok(output)
    })
}

fn join(drain: Option<Drain>) -> Result<rust_alloc::vec::Vec<u8>> {
    let Some(drain) = drain else {
        return Ok(rust_alloc::vec::Vec::new());
    };

    match drain.join() {
        Ok(output) => Ok(output?),
        Err(..) => Err(anyhow!("reading the output of cargo failed")),
    }
}

/// The declaration sources named by the packages in the output of `cargo
/// metadata`, each once and in a stable order.
///
/// A package whose `[package.metadata.rune]` is malformed is reported in
/// `warnings` and skipped, so that one crate does not cost the others theirs.
pub(super) fn declarations(
    metadata: &[u8],
    warnings: &mut Vec<crate::alloc::String>,
) -> Result<Vec<PathBuf>> {
    let metadata: serde_json::Value =
        serde_json::from_slice(metadata).context("parsing the output of cargo metadata")?;

    let Some(packages) = metadata.get("packages").and_then(|p| p.as_array()) else {
        bail!("the output of cargo metadata has no packages");
    };

    let mut output = Vec::new();

    for package in packages {
        let name = package.get("name").and_then(|n| n.as_str()).unwrap_or("?");

        let Some(rune) = package.get("metadata").and_then(|m| m.get("rune")) else {
            continue;
        };

        let Some(root) = package
            .get("manifest_path")
            .and_then(|p| p.as_str())
            .and_then(|p| Path::new(p).parent())
        else {
            warnings.try_push(crate::alloc::try_format!(
                "cargo package `{name}` has no manifest path"
            ))?;
            continue;
        };

        let patterns = match rune.get("declarations") {
            None => continue,
            Some(serde_json::Value::Array(patterns)) => patterns,
            Some(..) => {
                warnings.try_push(crate::alloc::try_format!(
                    "cargo package `{name}`: `package.metadata.rune.declarations` should be a list of paths"
                ))?;
                continue;
            }
        };

        for pattern in patterns {
            let Some(pattern) = pattern.as_str() else {
                warnings.try_push(crate::alloc::try_format!(
                    "cargo package `{name}`: `package.metadata.rune.declarations` should be a list of paths"
                ))?;
                continue;
            };

            let pattern = RelativePath::new(pattern);
            let glob = Glob::new(root, pattern)?;

            for path in glob.matcher()? {
                let path = match path {
                    Ok(path) => path,
                    Err(error) => {
                        warnings.try_push(crate::alloc::try_format!(
                            "cargo package `{name}`: {pattern}: {error}"
                        ))?;
                        continue;
                    }
                };

                if path.is_file() && !output.contains(&path) {
                    output.try_push(path)?;
                }
            }
        }
    }

    output.sort();
    Ok(output)
}
