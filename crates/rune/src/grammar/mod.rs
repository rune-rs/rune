//! This is the lossless and more relaxed parser for Rune.
//!
//! This produces a syntax tree that can be analysed using the provided methods.

mod grammar;
pub(crate) use self::grammar::root;

mod parser;
use self::parser::Checkpoint;
pub(crate) use self::parser::Parser;

mod tree;
pub(crate) use self::tree::{Node, Remaining, Stream, Tree};
