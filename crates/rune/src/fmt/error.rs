/*!
Error types for the formatting functionality.
 */

use crate::no_std as std;
use crate::no_std::io;
use crate::no_std::thiserror;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("I/O error")]
    IOError(#[from] io::Error),

    #[error("Invalid span: {0}..{1} but max is {2}")]
    InvalidSpan(usize, usize, usize),

    #[error("Error while parsing source")]
    CompileError(#[from] crate::compile::CompileError),

    #[error("Unexpected end of input")]
    Eof,
}
