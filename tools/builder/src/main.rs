//! A utility project for building and packaging Rune binaries.

use anyhow::{anyhow, bail, Context as _, Result};
use regex::Regex;
use std::env;
use std::env::consts;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<()> {
    let mut it = env::args();
    it.next();

    let mut channel = None::<Box<str>>;

    while let Some(args) = it.next() {
        match args.as_str() {
            "--channel" => {
                let name = it
                    .next()
                    .ok_or_else(|| anyhow!("expected argument to --channel"))?;
                channel = Some(name.into_boxed_str())
            }
            other => {
                bail!("Unsupported option `{}`", other);
            }
        }
    }

    let build = if let Some(channel) = channel {
        Build::Channel(channel)
    } else {
        let version = github_ref_version()?;
        env::set_var("RUNE_VERSION", &version);
        Build::Version(version)
    };

    if cfg!(target_os = "windows") {
        do_build(build, "windows", ".exe")?;
    } else if cfg!(target_os = "linux") {
        do_build(build, "linux", "")?;
    } else if cfg!(target_os = "macos") {
        do_build(build, "macos", "")?;
    } else {
        bail!("unsupported operating system: {}", consts::OS);
    }

    Ok(())
}

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

fn create_release_zip<I, V>(dest: &Path, version: V, sources: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
    V: fmt::Display,
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

fn create_zip<I>(file: &Path, sources: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let out = fs::File::create(file)?;
    let mut zip = zip::ZipWriter::new(out);

    for p in sources {
        let p = p.as_ref();
        println!("{}: adding: {}", file.display(), p.display());

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

fn create_gz(output: &Path, input: &Path) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    println!("building: {}", output.display());

    let input = fs::File::open(input)?;
    let output = fs::File::create(output)?;

    let mut input = io::BufReader::new(input);
    let mut encoder = GzEncoder::new(output, Compression::default());

    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

/// Copy an iterator of files to the given directory.
fn copy_files<I, S, N>(dest: &Path, sources: I) -> Result<()>
where
    I: IntoIterator<Item = (S, N)>,
    S: AsRef<Path>,
    N: AsRef<str>,
{
    for (s, name) in sources {
        let s = s.as_ref();
        let name = name.as_ref();

        fs::copy(s, dest.join(name))?;
    }

    Ok(())
}

fn do_build(build: Build, suffix: &str, ext: &str) -> Result<()> {
    let readme = PathBuf::from("README.md");
    let release_dir = PathBuf::from("target").join("release");
    let upload = PathBuf::from("target").join("upload");

    if !upload.is_dir() {
        fs::create_dir_all(&upload).context("creating upload directory")?;
    }

    let rune = release_dir.join(format!("rune{}", ext));
    let rune_languageserver = release_dir.join(format!("rune-languageserver{}", ext));

    if !rune.is_file() {
        println!("building: {}", rune.display());
        cargo(&["build", "--release", "--bin", "rune"]).context("building rune")?;
    }

    if !rune_languageserver.is_file() {
        println!("building: {}", rune_languageserver.display());
        cargo(&["build", "--release", "--bin", "rune-languageserver"])
            .context("building rune-languageserver")?;
    }

    // Create a zip file containing everything related to rune.
    create_release_zip(&upload, &build, vec![&readme, &rune, &rune_languageserver])
        .context("building .zip")?;

    if build.is_channel() {
        // Create rune-languageserver gzip.
        create_gz(
            &upload.join(format!("rune-languageserver-{}.gz", consts::OS)),
            &rune_languageserver,
        )
        .context("building rune-languageserver .gz")?;

        // Copy files to be uploaded.
        copy_files(
            &upload,
            vec![(
                rune_languageserver,
                format!("rune-languageserver-{}{}", suffix, ext),
            )],
        )
        .context("copying raw files to upload")?;
    }

    Ok(())
}

enum Build {
    Channel(Box<str>),
    Version(Version),
}

impl Build {
    /// Test if the build is a channel.
    fn is_channel(&self) -> bool {
        match self {
            Self::Channel(..) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Build {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Channel(channel) => write!(f, "{}", channel)?,
            Self::Version(version) => write!(f, "{}", version)?,
        }

        Ok(())
    }
}
