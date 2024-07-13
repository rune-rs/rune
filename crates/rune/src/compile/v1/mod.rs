pub(crate) mod assemble;
pub(crate) use self::assemble::Ctxt;

mod loops;
pub(crate) use self::loops::{Loop, Loops};

mod scopes;
use self::scopes::Linear;
pub(crate) use self::scopes::{Scope, ScopeId, Scopes};

mod slots;
use self::slots::Slots;

mod needs;
use self::needs::{Needs, NeedsAddress, NeedsAddressKind};
