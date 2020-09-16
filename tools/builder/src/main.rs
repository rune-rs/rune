use anyhow::{anyhow, bail, Result};
use regex::Regex;
use std::env;
use std::env::consts;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
struct Version {
    base: String,
    major: u32,
    minor: u32,
    patch: u32,
    pre: Option<u32>,
}

impl Version {
    /// Open a version by matching it against the given string.
    pub fn open(version: impl AsRef<str>) -> Result<Option<Version>> {
        let version_re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(-.+\.(\d+))?$")?;
        let version = version.as_ref();

        let m = match version_re.captures(version) {
            Some(m) => m,
            None => return Ok(None),
        };

        let major: u32 = str::parse(&m[1])?;
        let minor: u32 = str::parse(&m[2])?;
        let patch: u32 = str::parse(&m[3])?;
        let pre: Option<u32> = m.get(5).map(|s| str::parse(s.as_str())).transpose()?;

        Ok(Some(Self {
            base: version.to_string(),
            major,
            minor,
            patch,
            pre,
        }))
    }
}

/// Get the version from GITHUB_REF.
fn github_ref_version() -> Result<Version> {
    let version = match env::var("GITHUB_REF") {
        Ok(version) => version,
        _ => bail!("missing: GITHUB_REF"),
    };

    let mut it = version.split('/');

    let version = match (it.next(), it.next(), it.next()) {
        (Some("refs"), Some("tags"), Some(version)) => {
            Version::open(version)?.ok_or_else(|| anyhow!("Expected valid version"))?
        }
        _ => bail!("expected GITHUB_REF: refs/tags/*"),
    };

    Ok(version)
}

impl fmt::Display for Version {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.fmt(fmt)
    }
}

impl AsRef<[u8]> for Version {
    fn as_ref(&self) -> &[u8] {
        self.base.as_bytes()
    }
}

impl AsRef<OsStr> for Version {
    fn as_ref(&self) -> &OsStr {
        self.base.as_ref()
    }
}

fn cargo(args: &[&str]) -> Result<()> {
    println!("cargo {}", args.join(" "));
    let status = Command::new("cargo").args(args).status()?;

    if !status.success() {
        bail!("failed to run cargo");
    }

    Ok(())
}

fn create_zip<I>(file: &Path, sources: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut zip = zip::ZipWriter::new(fs::File::create(file)?);

    for p in sources {
        let p = p.as_ref();
        println!("Adding to zip: {}", p.display());

        let file_name = p
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| anyhow!("file name is not a string"))?;

        zip.start_file(file_name, options)?;
        let mut from = fs::File::open(&p)?;
        io::copy(&mut from, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}

/// Copy an iterator of files to the given directory.
fn copy_files<I>(dest: &Path, sources: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    for s in sources {
        let s = s.as_ref();

        if let Some(name) = s.file_name() {
            fs::copy(s, dest.join(name))?;
        }
    }

    Ok(())
}

/// Create a zip distribution.
fn create_zip_dist<I>(dest: &Path, version: &Version, sources: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    if !dest.is_dir() {
        fs::create_dir_all(dest)?;
    }

    let zip_file = dest.join(format!(
        "rune-{version}-{os}-{arch}.zip",
        version = version,
        os = consts::OS,
        arch = consts::ARCH
    ));

    println!("Creating Zip File: {}", zip_file.display());
    create_zip(&zip_file, sources)?;
    Ok(())
}

fn build(root: &Path, ext: &str) -> Result<()> {
    let version = github_ref_version()?;

    env::set_var("RUNE_VERSION", &version);

    println!("version: {}", version);

    let readme = root.join("README.md");
    let release_dir = root.join("target").join("release");
    let upload = root.join("target").join("upload");

    let rune = release_dir.join(format!("rune{}", ext));
    let rune_languageserver = release_dir.join(format!("rune-languageserver{}", ext));

    if !rune.is_file() {
        println!("building: {}", rune.display());
        cargo(&["build", "--release", "-p", "rune"])?;
    }

    if !rune_languageserver.is_file() {
        println!("building: {}", rune_languageserver.display());
        cargo(&["build", "--release", "-p", "rune-languageserver"])?;
    }

    create_zip_dist(
        &upload,
        &version,
        vec![&readme, &rune, &rune_languageserver],
    )?;

    copy_files(&upload, vec![&rune_languageserver])?;
    Ok(())
}

/// Perform a Windows build.
fn windows_build(root: &Path) -> Result<()> {
    build(root, ".exe")?;
    Ok(())
}

/// Perform a Linux build.
fn linux_build(root: &Path) -> Result<()> {
    build(root, "")?;
    Ok(())
}

/// Perform a MacOS build.
fn macos_build(root: &Path) -> Result<()> {
    build(root, "")?;
    Ok(())
}

fn main() -> Result<()> {
    let root = env::current_dir()?;
    println!("root: {}", root.display());

    if cfg!(target_os = "windows") {
        windows_build(&root)?;
    } else if cfg!(target_os = "linux") {
        linux_build(&root)?;
    } else if cfg!(target_os = "macos") {
        macos_build(&root)?;
    } else {
        bail!("unsupported operating system: {}", consts::OS);
    }

    Ok(())
}
