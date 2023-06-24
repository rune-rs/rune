use core::mem::replace;

use std::collections::HashSet;
use std::ffi::OsStr;

use crate::no_std::prelude::*;

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
    pub(crate) fn name(&mut self, e: &EntryPoint<'_>) -> String {
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
        if !self.names.insert(name.clone()) {
            let next = self.count.wrapping_add(1);
            let index = replace(&mut self.count, next);
            format!("{name}{index}")
        } else {
            name
        }
    }
}
