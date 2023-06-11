use crate::no_std::prelude::*;

pub(crate) mod assemble;
mod loops;
mod scopes;

use crate::Hash;

pub(crate) use self::assemble::{Ctxt, Needs};
pub(crate) use self::loops::{Loop, Loops};
pub(crate) use self::scopes::{Layer, ScopeGuard, Scopes, Var};

/// Generic parameters.
#[derive(Default)]
pub(crate) struct GenericsParameters {
    pub(crate) trailing: usize,
    pub(crate) parameters: [Option<Hash>; 2],
}

impl GenericsParameters {
    pub(crate) fn is_empty(&self) -> bool {
        self.parameters.iter().all(|p| p.is_none())
    }

    pub(crate) fn as_boxed(&self) -> Box<[Option<Hash>]> {
        self.parameters.iter().copied().collect()
    }
}

impl AsRef<GenericsParameters> for GenericsParameters {
    #[inline]
    fn as_ref(&self) -> &GenericsParameters {
        self
    }
}
