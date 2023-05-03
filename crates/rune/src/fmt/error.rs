/*!
Error types for the formatting functionality.
 */

use crate::no_std as std;
use crate::no_std::io;
use crate::no_std::thiserror;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("io error")]
    IOError(#[from] io::Error),

    #[error("invalid span: {0}..{1} but max is {2}")]
    InvalidSpan(usize, usize, usize),

    #[error("error while parsing source")]
    ParseError(#[from] crate::parse::ParseError),

    #[error("unexpected end of input")]
    Eof,
}
