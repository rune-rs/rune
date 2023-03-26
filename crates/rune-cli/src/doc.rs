use std::io::Write;
use std::path::Path;

use anyhow::Context;
use rune::compile::FileSourceLoader;
use rune::{Diagnostics, Options, Source, Sources};
use structopt::StructOpt;

use crate::{Config, ExitCode, Io, SharedFlags};

#[derive(StructOpt, Debug, Clone)]
pub(crate) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[structopt(long)]
    warnings_are_errors: bool,

    #[structopt(flatten)]
    pub(crate) shared: SharedFlags,
}

pub(crate) fn run(
    io: &mut Io<'_>,
    c: &Config,
    flags: &Flags,
    options: &Options,
    path: &Path,
) -> anyhow::Result<ExitCode> {
    writeln!(io.stdout, "Building documentation: {}", path.display())?;

    let context = flags.shared.context(c)?;

    let source =
        Source::from_path(path).with_context(|| format!("reading file: {}", path.display()))?;

    let mut sources = Sources::new();

    sources.insert(source);

    let mut diagnostics = if flags.shared.warnings || flags.warnings_are_errors {
        Diagnostics::new()
    } else {
        Diagnostics::without_warnings()
    };

    let mut visitor = rune::doc::Visitor::default();
    let mut source_loader = FileSourceLoader::new();

    let _ = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(options)
        .with_visitor(&mut visitor)
        .with_source_loader(&mut source_loader)
        .build();

    diagnostics.emit(&mut io.stdout.lock(), &sources)?;

    let root = Path::new("target/rune-doc");
    rune::doc::write_html("script", root, &context, &visitor)?;

    if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
        Ok(ExitCode::Failure)
    } else {
        Ok(ExitCode::Success)
    }
}
