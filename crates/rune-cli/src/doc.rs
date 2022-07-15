use std::collections::BTreeMap;
use std::io::Write;
use std::{collections::HashMap, path::Path};

use anyhow::Context;
use rune::compile::MetaKind;
use rune::{
    ast::Span,
    compile::{CompileVisitor, FileSourceLoader, Item, MetaRef},
    Diagnostics, Options, Source, SourceId, Sources,
};
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

    let mut doc_finder = DocFinder::default();
    let mut source_loader = FileSourceLoader::new();

    let _ = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(options)
        .with_visitor(&mut doc_finder)
        .with_source_loader(&mut source_loader)
        .build();

    diagnostics.emit(&mut io.stdout.lock(), &sources)?;

    for (item, kind) in &doc_finder.meta {
        writeln!(io.stdout, "{item}: {kind:?}")?;

        if let Some(doc) = doc_finder.docs.get(item) {
            for line in doc {
                writeln!(io.stdout, "{:?}", line)?;
            }
        }
    }

    if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
        Ok(ExitCode::Failure)
    } else {
        Ok(ExitCode::Success)
    }
}

#[derive(Default)]
struct DocFinder {
    meta: BTreeMap<Item, MetaKind>,
    docs: HashMap<Item, Vec<String>>,
}

impl CompileVisitor for DocFinder {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        self.meta.insert(meta.item.clone(), meta.kind);
    }

    fn visit_doc_comment(&mut self, _source_id: SourceId, item: &Item, _span: Span, string: &str) {
        self.docs
            .entry(item.clone())
            .or_default()
            .push(string.to_owned());
    }
}
