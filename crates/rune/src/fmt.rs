//! Helper to format Rune code.

mod error;
mod indent_writer;
mod printer;

use crate::{
    ast,
    parse::{Parse, Parser},
    Source, SourceId,
};
use printer::Printer;
use std::io::Cursor;

use self::error::FormattingError;

/// Format the given contents.
pub fn layout(contents: String) -> Result<String, FormattingError> {
    let mut parser = Parser::new(&contents, SourceId::new(0), true);

    let ast = ast::File::parse(&mut parser).unwrap();
    let s = Source::new("xx", &contents);
    let mut buf: Vec<u8> = vec![];
    let mut printer: Printer<Cursor<&mut Vec<u8>>> = Printer::new(Cursor::new(&mut buf), &s);

    printer.visit_file(&ast)?;
    let res = String::from_utf8(buf).unwrap().trim().to_owned() + "\n";
    Ok(res)
}
