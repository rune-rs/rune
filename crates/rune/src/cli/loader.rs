use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Context as _, Result};

use crate::alloc::{Vec, VecDeque};
use crate::cli::{visitor, Io, SharedFlags};
use crate::compile::{FileSourceLoader, ItemBuf};
use crate::Diagnostics;
use crate::{Context, Hash, Options, Source, Sources, Unit};

pub(super) struct Load {
    pub(super) unit: Arc<Unit>,
    pub(super) sources: Sources,
    pub(super) functions: Vec<(Hash, ItemBuf)>,
}

/// Load context and code for a given path
pub(super) fn load(
    io: &mut Io<'_>,
    context: &Context,
    shared: &SharedFlags,
    options: &Options,
    path: &Path,
    attribute: visitor::Attribute,
) -> Result<Load> {
    let bytecode_path = path.with_extension("rnc");

    let source =
        Source::from_path(path).with_context(|| anyhow!("cannot read file: {}", path.display()))?;

    let mut sources = Sources::new();
    sources.insert(source)?;

    let use_cache = options.bytecode && should_cache_be_used(path, &bytecode_path)?;

    // TODO: how do we deal with tests discovery for bytecode loading
    let maybe_unit = if use_cache {
        let f = fs::File::open(&bytecode_path)?;

        match bincode::deserialize_from::<_, Unit>(f) {
            Ok(unit) => {
                tracing::trace!("Using cache: {}", bytecode_path.display());
                Some(Arc::new(unit))
            }
            Err(e) => {
                tracing::error!("Failed to deserialize: {}: {}", bytecode_path.display(), e);
                None
            }
        }
    } else {
        None
    };

    let (unit, functions) = match maybe_unit {
        Some(unit) => (unit, Default::default()),
        None => {
            tracing::trace!("building file: {}", path.display());

            let mut diagnostics = if shared.warnings {
                Diagnostics::new()
            } else {
                Diagnostics::without_warnings()
            };

            let mut functions = visitor::FunctionVisitor::new(attribute);
            let mut source_loader = FileSourceLoader::new();

            let result = crate::prepare(&mut sources)
                .with_context(context)
                .with_diagnostics(&mut diagnostics)
                .with_options(options)
                .with_visitor(&mut functions)?
                .with_source_loader(&mut source_loader)
                .build();

            diagnostics.emit(io.stdout, &sources)?;
            let unit = result?;

            if options.bytecode {
                tracing::trace!("serializing cache: {}", bytecode_path.display());
                let f = fs::File::create(&bytecode_path)?;
                bincode::serialize_into(f, &unit)?;
            }

            (Arc::new(unit), functions.into_functions())
        }
    };

    Ok(Load {
        unit,
        sources,
        functions,
    })
}

/// Test if path `a` is newer than path `b`.
fn should_cache_be_used(source: &Path, cached: &Path) -> io::Result<bool> {
    let source = fs::metadata(source)?;

    let cached = match fs::metadata(cached) {
        Ok(cached) => cached,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };

    Ok(source.modified()? < cached.modified()?)
}

pub(super) fn recurse_paths(
    recursive: bool,
    first: PathBuf,
) -> impl Iterator<Item = Result<PathBuf>> {
    let mut queue = VecDeque::new();
    let mut first = Some(first);

    std::iter::from_fn(move || loop {
        let path = first.take().or_else(|| queue.pop_front())?;

        if !recursive {
            return Some(Ok(path));
        }

        if path.is_file() {
            if path.extension() == Some(OsStr::new("rn")) {
                return Some(Ok(path));
            }

            continue;
        }

        let d = match fs::read_dir(path) {
            Ok(d) => d,
            Err(error) => return Some(Err(anyhow::Error::from(error))),
        };

        for e in d {
            let e = match e {
                Ok(e) => e,
                Err(error) => return Some(Err(anyhow::Error::from(error))),
            };

            if let Err(error) = queue.try_push_back(e.path()) {
                return Some(Err(anyhow::Error::from(error)));
            }
        }
    })
}
