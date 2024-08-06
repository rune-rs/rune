use std::io::Write;
use std::path::PathBuf;

use crate::doc::Artifacts;

use anyhow::{Context, Result};
use clap::Parser;

use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::cli::naming::Naming;
use crate::cli::{AssetKind, CommandBase, Config, Entry, EntryPoint, ExitCode, Io, SharedFlags};
use crate::compile::FileSourceLoader;
use crate::{Diagnostics, Options, Source, Sources};

#[derive(Parser, Debug)]
pub(super) struct Flags {
    /// Output directory to write ace extensions to.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Generate .await and ? extension for functions.
    #[arg(long)]
    extensions: bool,
    /// Do not include `rune-mode.js`.
    #[arg(long)]
    no_mode: bool,
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,
}

impl CommandBase for Flags {
    #[inline]
    fn is_workspace(&self, _: AssetKind) -> bool {
        true
    }

    #[inline]
    fn describe(&self) -> &str {
        "Documenting"
    }
}

pub(super) fn run<'p, I>(
    io: &mut Io<'_>,
    entry: &mut Entry<'_>,
    c: &Config,
    flags: &Flags,
    shared: &SharedFlags,
    options: &Options,
    entries: I,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'p>>,
{
    let root = match &flags.output {
        Some(root) => root.clone(),
        None => match &c.manifest_root {
            Some(path) => path.join("target").join("rune-ace"),
            None => match std::env::var_os("CARGO_TARGET_DIR") {
                Some(target) => {
                    let mut target = PathBuf::from(target);
                    target.push("rune-ace");
                    target
                }
                None => {
                    let mut target = PathBuf::new();
                    target.push("target");
                    target.push("rune-ace");
                    target
                }
            },
        },
    };

    writeln!(io.stdout, "Building ace autocompletion: {}", root.display())?;

    let context = shared.context(entry, c, None)?;

    let mut visitors = Vec::new();

    let mut naming = Naming::default();

    for e in entries {
        let item = naming.item(&e)?;

        let mut visitor = crate::doc::Visitor::new(&item)?;
        let mut sources = Sources::new();

        let source = match Source::from_path(e.path()) {
            Ok(source) => source,
            Err(error) => return Err(error).context(e.path().display().try_to_string()?),
        };

        sources.insert(source)?;

        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut source_loader = FileSourceLoader::new();

        let _ = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(options)
            .with_visitor(&mut visitor)?
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            return Ok(ExitCode::Failure);
        }

        visitors.try_push(visitor)?;
    }

    let mut artifacts = Artifacts::new();
    crate::ace::build_autocomplete(&mut artifacts, &context, &visitors, flags.extensions)?;

    if !flags.no_mode {
        crate::ace::theme(&mut artifacts)?;
    }

    for asset in artifacts.assets() {
        asset.build(&root)?;
    }

    Ok(ExitCode::Success)
}
