//! Helper to format Rune code.

mod comments;
mod error;
mod indent_writer;
mod printer;
mod whitespace;

use crate::{
    ast,
    parse::{Parse, Parser},
    Source, SourceId,
};
use printer::Printer;

use self::error::FormattingError;

/// Format the given contents.
pub fn layout_string(contents: String) -> Result<String, FormattingError> {
    let s = Source::new("<memory>", contents);
    layout_source(&s)
}

/// Format the given source.
pub fn layout_source(source: &Source) -> Result<String, FormattingError> {
    let mut parser = Parser::new(source.as_str(), SourceId::new(0), true);

    let ast = ast::File::parse(&mut parser).unwrap();
    let mut printer: Printer = Printer::new(source);

    printer.visit_file(&ast)?;

    let res = printer.commit().trim().to_owned() + "\n";

    Ok(res)
}
