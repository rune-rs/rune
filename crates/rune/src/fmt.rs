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
use std::io::Cursor;

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
    let mut buf: Vec<u8> = vec![];
    let mut printer: Printer<Cursor<&mut Vec<u8>>> = Printer::new(Cursor::new(&mut buf), source);

    printer.visit_file(&ast)?;
    let res = String::from_utf8(buf).unwrap().trim().to_owned() + "\n";

    Ok(res)
}
