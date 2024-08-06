use std::ffi::OsStr;

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::cli::EntryPoint;
use crate::item::ComponentRef;
use crate::ItemBuf;

/// Helper to perform non-conflicting crate naming.
#[derive(Default)]
pub(crate) struct Naming {
    names: HashMap<ItemBuf, usize>,
}

impl Naming {
    /// Construct a unique crate name for the given entrypoint.
    pub(crate) fn item(&mut self, e: &EntryPoint<'_>) -> alloc::Result<ItemBuf> {
        let mut item = match &e {
            EntryPoint::Path(path, _) => match path.file_stem().and_then(OsStr::to_str) {
                Some(name) => ItemBuf::with_crate(name)?,
                None => ItemBuf::with_crate("entry")?,
            },
            EntryPoint::Package(p) => {
                let name = p.found.name.as_str();
                ItemBuf::with_crate_item(&p.package.name, [name])?
            }
        };

        let values = self.names.entry(item.try_clone()?).or_try_default()?;

        if *values > 0 {
            let name = try_format!("{}", *values - 1);
            item.push(ComponentRef::Str(&name))?;
        }

        *values += 1;
        Ok(item)
    }
}
