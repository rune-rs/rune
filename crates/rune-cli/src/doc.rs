use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{self, Write};
use std::path::Path;

use anyhow::Context;
use rune::compile::{
    CompileVisitor, Component, FileSourceLoader, Item, ItemBuf, Location, MetaKind, MetaRef,
};
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

    let mut queue = VecDeque::new();
    queue.push_back(ItemBuf::new());
    walk_items(io, &doc_finder, &mut queue)?;

    let root = Path::new("target/rune-doc");
    rune::doc::write_html(&context, root)?;

    let mut it = context.iter_meta().peekable();

    while let Some((item, meta)) = it.next() {
        if !meta.docs.is_empty() {
            let args = match meta.docs.args() {
                Some(args) => args.join(", "),
                None => String::from("?"),
            };

            writeln!(io.stdout, "fn {item}({args}):")?;
            writeln!(io.stdout)?;

            for line in meta.docs.lines() {
                writeln!(io.stdout, "  {line}", line = line.trim())?;
            }

            if it.peek().is_some() {
                writeln!(io.stdout)?;
            }
        }
    }

    if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
        Ok(ExitCode::Failure)
    } else {
        Ok(ExitCode::Success)
    }
}

/// Walk items.
fn walk_items(io: &mut Io<'_>, doc: &DocFinder, queue: &mut VecDeque<ItemBuf>) -> io::Result<()> {
    while let Some(current) = queue.pop_front() {
        writeln!(io.stdout, "module: {}", &current)?;

        for c in doc.structs.get(&current).into_iter().flatten() {
            let struct_key = current.join(&[c.as_component_ref()]);
            if let Some(docs) = doc.field_docs.get(&struct_key) {
                writeln!(io.stdout, "struct {} {{", c)?;
                for field in docs.keys() {
                    writeln!(io.stdout, "  {}", field)?;
                }

                writeln!(io.stdout, "}}")?;
            } else {
                writeln!(io.stdout, "struct {}", c)?;
            }
        }

        for c in doc.enums.get(&current).into_iter().flatten() {
            let item = current.join(&[c.as_component_ref()]);

            writeln!(io.stdout, "enum {} {{", c)?;

            for c in doc.variants.get(&item).into_iter().flatten() {
                let struct_key = item.join(&[c.as_component_ref()]);
                if let Some(docs) = doc.field_docs.get(&struct_key) {
                    writeln!(io.stdout, "  {} {{", c)?;
                    for field in docs.keys() {
                        writeln!(io.stdout, "    {}", field)?;
                    }

                    writeln!(io.stdout, "  }}")?;
                } else {
                    writeln!(io.stdout, "  {}", c)?;
                }
            }

            writeln!(io.stdout, "}}")?;
        }

        for module in doc.modules.get(&current).into_iter().flatten() {
            let item = current.join(&[module.as_component_ref()]);
            queue.push_back(item);
        }
    }

    Ok(())
}

#[derive(Default)]
struct DocFinder {
    meta: BTreeMap<ItemBuf, MetaKind>,
    docs: HashMap<ItemBuf, Vec<String>>,
    field_docs: HashMap<ItemBuf, HashMap<Box<str>, Vec<String>>>,
    modules: BTreeMap<ItemBuf, BTreeSet<Component>>,
    structs: BTreeMap<ItemBuf, BTreeSet<Component>>,
    enums: BTreeMap<ItemBuf, BTreeSet<Component>>,
    variants: BTreeMap<ItemBuf, BTreeSet<Component>>,
}

impl CompileVisitor for DocFinder {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        self.meta.insert(meta.item.to_owned(), meta.kind);

        let parent = meta.item.parent().unwrap_or_default();

        match meta.kind {
            MetaKind::Module => {
                if let Some(name) = meta.item.last() {
                    self.modules
                        .entry(parent.to_owned())
                        .or_default()
                        .insert(name.to_owned());
                }
            }
            MetaKind::Enum => {
                if let Some(name) = meta.item.last() {
                    self.enums
                        .entry(parent.to_owned())
                        .or_default()
                        .insert(name.to_owned());
                }
            }
            MetaKind::UnitStruct | MetaKind::TupleStruct | MetaKind::Struct => {
                if let Some(name) = meta.item.last() {
                    self.structs
                        .entry(parent.to_owned())
                        .or_default()
                        .insert(name.to_owned());
                }
            }
            MetaKind::UnitVariant | MetaKind::TupleVariant | MetaKind::StructVariant => {
                if let Some(name) = meta.item.last() {
                    self.variants
                        .entry(parent.to_owned())
                        .or_default()
                        .insert(name.to_owned());
                }
            }
            _ => {}
        }
    }

    fn visit_doc_comment(&mut self, _location: Location, item: &Item, string: &str) {
        self.docs
            .entry(item.to_owned())
            .or_default()
            .push(string.to_owned());
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        item: &Item,
        field: &str,
        string: &str,
    ) {
        let map = self.field_docs.entry(item.to_owned()).or_default();
        map.entry(field.into()).or_default().push(string.to_owned());
    }
}
