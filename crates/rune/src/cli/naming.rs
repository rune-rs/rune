use core::mem::replace;

use std::ffi::OsStr;

use crate::alloc::prelude::*;
use crate::alloc::{self, try_format, HashSet, String};
use crate::cli::EntryPoint;
use crate::workspace;

/// Helper to perform non-conflicting crate naming.
#[derive(Default)]
pub(crate) struct Naming {
    names: HashSet<String>,
    count: usize,
}

impl Naming {
    /// Construct a unique crate name for the given entrypoint.
    pub(crate) fn name(&mut self, e: &EntryPoint<'_>) -> alloc::Result<String> {
        let name = match &e {
            EntryPoint::Path(path) => match path.file_stem().and_then(OsStr::to_str) {
                Some(name) => String::try_from(name)?,
                None => String::try_from("entry")?,
            },
            EntryPoint::Package(p) => {
                let name = p.found.name.as_str();

                let ext = match &p.found.kind {
                    workspace::FoundKind::Binary => "bin",
                    workspace::FoundKind::Test => "test",
                    workspace::FoundKind::Example => "example",
                    workspace::FoundKind::Bench => "bench",
                };

                try_format!("{}-{name}-{ext}", p.package.name)
            }
        };

        // TODO: make it so that we can communicate different entrypoints in the
        // visitors context instead of this hackery.
        Ok(if !self.names.try_insert(name.try_clone()?)? {
            let next = self.count.wrapping_add(1);
            let index = replace(&mut self.count, next);
            try_format!("{name}{index}")
        } else {
            name
        })
    }
}
