pub(crate) mod assemble;
pub(crate) use self::assemble::Ctxt;

mod loops;
pub(crate) use self::loops::{Loop, Loops};

mod scopes;
pub(crate) use self::scopes::Scopes;
use self::scopes::{Linear, ScopeId};

mod slots;
use self::slots::Slots;

mod needs;
use self::needs::{Needs, NeedsAddress, NeedsAddressKind};

mod slab;
use self::slab::Slab;
