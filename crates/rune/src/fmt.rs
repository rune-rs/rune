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

/// Format the given contents.
pub fn layout_string(contents: String) -> Result<String, FormattingError> {
    let s = Source::new("<memory>", contents);
    layout_source(&s)
}

/// Format the given source.
pub fn layout_source(source: &Source) -> Result<String, FormattingError> {
    let mut parser = Parser::new(source.as_str(), SourceId::new(0), true);

    let ast = ast::File::parse(&mut parser)?;
    let mut printer: Printer = Printer::new(source)?;

    printer.visit_file(&ast)?;

    let res = printer.commit().trim().to_owned() + "\n";

    Ok(res)
}
