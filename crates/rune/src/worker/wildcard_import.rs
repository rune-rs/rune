use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::compile::{self, ErrorKind, IntoComponent, ItemBuf, Location, ModId, Visibility};
use crate::query::Query;

pub(crate) struct WildcardImport {
    pub(crate) visibility: Visibility,
    pub(crate) from: ItemBuf,
    pub(crate) name: ItemBuf,
    pub(crate) location: Location,
    pub(crate) module: ModId,
    pub(crate) found: bool,
}

impl WildcardImport {
    pub(crate) fn process_global(&mut self, query: &mut Query<'_, '_>) -> compile::Result<()> {
        if query.context.contains_prefix(&self.name)? {
            for c in query.context.iter_components(&self.name)? {
                let name = self.name.extended(c)?;

                query.insert_import(
                    &self.location,
                    self.module,
                    self.visibility,
                    self.from.try_clone()?,
                    name,
                    None,
                    true,
                )?;
            }

            self.found = true;
        }

        Ok(())
    }

    /// Process a local wildcard import.
    pub(crate) fn process_local(&mut self, query: &mut Query) -> compile::Result<()> {
        if query.contains_prefix(&self.name)? {
            let components = query
                .iter_components(&self.name)?
                .map(|c| c.into_component())
                .try_collect::<Result<Vec<_>, _>>()??;

            for c in components {
                let name = self.name.extended(c)?;

                query.insert_import(
                    &self.location,
                    self.module,
                    self.visibility,
                    self.from.try_clone()?,
                    name,
                    None,
                    true,
                )?;
            }

            self.found = true;
        }

        if !self.found {
            return Err(compile::Error::new(
                self.location,
                ErrorKind::MissingItem {
                    item: self.name.try_clone()?,
                },
            ));
        }

        Ok(())
    }
}
