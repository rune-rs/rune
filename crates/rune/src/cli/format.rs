use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::{Config, Entry, EntryPoint, ExitCode, Io, SharedFlags};
use crate::compile::FileSourceLoader;
use crate::{Diagnostics, Options, Source, Sources};

#[derive(Parser, Debug, Clone)]
pub(super) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,

    #[command(flatten)]
    pub(super) shared: SharedFlags,
}

pub(super) fn run(
    io: &mut Io<'_>,
    entry: &mut Entry<'_>,
    c: &Config,
    flags: &Flags,
    options: &Options,
    path: &Path,
) -> Result<ExitCode> {
    let source =
        Source::from_path(path).with_context(|| format!("reading file: {}", path.display()))?;

    let formatted = crate::fmt::layout_source(&source)?;

    if formatted == source.as_str() {
        println!("{} already formatted", path.display());
    } else {
        println!("{} formatted", path.display());
    }

    std::fs::write(path, &formatted)?;

    Ok(ExitCode::Success)
}
