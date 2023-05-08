use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;

use crate::no_std::prelude::*;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::{Config, Entry, EntryPoint, ExitCode, Io, SharedFlags};
use crate::compile::{FileSourceLoader, ItemBuf};
use crate::{Diagnostics, Options, Source, Sources, workspace};

#[derive(Parser, Debug)]
pub(super) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,
    /// Output directory to write documentation to.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Open the generated documentation in a browser.
    #[arg(long)]
    open: bool,
}

pub(super) fn run<'p, I>(
    io: &mut Io<'_>,
    entry: &mut Entry<'_>,
    c: &Config,
    flags: &Flags,
    shared: &SharedFlags,
    options: &Options,
    entrys: I,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'p>>,
{
    let root = match &flags.output {
        Some(root) => root.to_owned(),
        None => match &c.manifest_root {
            Some(path) => path.join("target").join("rune-doc"),
            None => match std::env::var_os("CARGO_TARGET_DIR") {
                Some(target) => {
                    let mut target = PathBuf::from(target);
                    target.push("rune-doc");
                    target
                }
                None => {
                    let mut target = PathBuf::new();
                    target.push("target");
                    target.push("rune-doc");
                    target
                }
            },
        },
    };

    writeln!(io.stdout, "Building documentation: {}", root.display())?;

    let context = shared.context(entry, c, None)?;

    let mut visitors = Vec::new();

    let mut names = HashSet::new();

    for (index, e) in entrys.into_iter().enumerate() {
        let name = match &e {
            EntryPoint::Path(path) => {
                match path.file_stem().and_then(OsStr::to_str) {
                    Some(name) => String::from(name),
                    None => String::from("entry"),
                }
            }
            EntryPoint::Package(p) => {
                let name = p.found.name.as_str();

                let ext = match &p.found.kind {
                    workspace::FoundKind::Binary => "bin",
                    workspace::FoundKind::Test => "test",
                    workspace::FoundKind::Example => "example",
                    workspace::FoundKind::Bench => "bench",
                };

                format!("{}-{name}-{ext}", p.package.name)
            },
        };

        // TODO: make it so that we can communicate different entrypoints in the
        // visitors context instead of this hackery.
        let name = if !names.insert(name.clone()) {
            format!("{name}{index}")
        } else {
            name
        };

        let item = ItemBuf::with_crate(&name);
        let mut visitor = crate::doc::Visitor::new(item);
        let mut sources = Sources::new();
        let source = Source::from_path(e.path())
            .with_context(|| e.path().display().to_string())?;
        sources.insert(source);

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
            .with_visitor(&mut visitor)
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            return Ok(ExitCode::Failure);
        }

        visitors.push(visitor);
    }

    crate::doc::write_html("root", &root, &context, &visitors)?;

    if flags.open {
        let path = root.join("index.html");
        let _ = webbrowser::open(&path.display().to_string());
    }

    Ok(ExitCode::Success)
}
