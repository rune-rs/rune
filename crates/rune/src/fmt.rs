//! Helper to format Rune code.

#[cfg(test)]
mod tests;

mod comments;
mod error;
mod indent_writer;
mod printer;
mod whitespace;

use crate::no_std::prelude::*;

use crate::ast;
use crate::parse::{Parse, Parser};
use crate::{Source, SourceId};

use self::error::FormattingError;
use self::printer::Printer;

/// Format the given source.
pub(crate) fn layout_source(source: &Source) -> Result<Vec<u8>, FormattingError> {
    let mut parser = Parser::new(source.as_str(), SourceId::new(0), true);

    let ast = ast::File::parse(&mut parser)?;
    let mut printer: Printer = Printer::new(source)?;
    printer.visit_file(&ast)?;
    printer.commit()
}
