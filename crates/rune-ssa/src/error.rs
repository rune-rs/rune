use crate::{BlockId, StaticId, Var};
use thiserror::Error;

/// Error raised during machine construction.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot jump {0} -> {0}: destination is sealed")]
    SealedBlockJump(BlockId, BlockId),
    #[error("missing phi node for static id {0}")]
    MissingPhiNode(StaticId),
    #[error("missing value for static id {0}")]
    MissingValue(StaticId),
    #[error("missing block with id {0}")]
    MissingBlock(BlockId),
    #[error("tried to construct a float constant that is NaN")]
    FloatIsNan,
    #[error("missing local assignment {0}")]
    MissingLocal(Var),
}
