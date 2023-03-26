use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rune::compile::FileSourceLoader;
use rune::{Diagnostics, Options, Source, Sources};

use crate::{Config, EntryPoint, ExitCode, Io, SharedFlags};

#[derive(Parser, Debug, Clone)]
pub(crate) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,
    /// Output directory to write documentation to.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Open the generated documentation in a browser.
    #[arg(long)]
    open: bool,
    #[command(flatten)]
    pub(crate) shared: SharedFlags,
}

pub(crate) fn run<I>(
    io: &mut Io<'_>,
    c: &Config,
    flags: &Flags,
    options: &Options,
    entrys: I,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint>,
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

    let context = flags.shared.context(c)?;

    let mut visitors = Vec::new();

    for e in entrys {
        let mut visitor = rune::doc::Visitor::new(e.item);
        let mut sources = Sources::new();

        for path in &e.paths {
            let source = Source::from_path(path)
                .with_context(|| format!("reading file: {}", path.display()))?;
            sources.insert(source);
        }

        let mut diagnostics = if flags.shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut source_loader = FileSourceLoader::new();

        let _ = rune::prepare(&mut sources)
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

    rune::doc::write_html("root", &root, &context, &visitors)?;

    if flags.open {
        let path = root.join("index.html");
        let _ = webbrowser::open(&path.display().to_string());
    }

    Ok(ExitCode::Success)
}
