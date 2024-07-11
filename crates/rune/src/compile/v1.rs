pub(crate) mod assemble;
pub(crate) use self::assemble::Ctxt;
use self::assemble::Needs;

mod loops;
pub(crate) use self::loops::{Loop, Loops};

mod scopes;
pub(crate) use self::scopes::{Scope, ScopeId, Scopes, Var};

mod slab;
use self::slab::Slab;
