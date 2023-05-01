// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2023, Tom Solberg, all rights reserved.
// Created: 27 April 2023

/*!
Error types for the formatting functionality.
 */

use std::io;
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
