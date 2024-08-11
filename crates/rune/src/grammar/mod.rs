//! This is the lossless and more relaxed parser for Rune.
//!
//! This produces a syntax tree that can be analysed using the provided methods.

mod grammar;
pub(crate) use self::grammar::{object_key, ws};

mod parser;
use self::parser::Checkpoint;

mod tree;
pub(crate) use self::tree::{Ignore, Node, Remaining, Stream, Tree};

use crate::macros::TokenStream;
use crate::parse::Lexer;
use crate::{compile, SourceId};

use self::parser::{Parser, Source};

/// Prepare parsing of text input.
pub(crate) fn prepare_text(input: &str) -> Prepare<'_> {
    Prepare::with_input(Input::Text(input))
}

/// Prepare parsing of a token stream.
#[allow(unused)]
pub(crate) fn prepare_token_stream(token_stream: &TokenStream) -> Prepare<'_> {
    Prepare::with_input(Input::TokenStream(token_stream))
}

enum Input<'a> {
    Text(&'a str),
    TokenStream(&'a TokenStream),
}

/// A prepared parse.
pub(crate) struct Prepare<'a> {
    input: Input<'a>,
    without_processing: bool,
    shebang: bool,
    source_id: SourceId,
}

impl<'a> Prepare<'a> {
    fn with_input(input: Input<'a>) -> Self {
        Self {
            input,
            without_processing: false,
            shebang: true,
            source_id: SourceId::new(0),
        }
    }

    /// Disable input processing.
    pub(crate) fn without_processing(mut self) -> Self {
        self.without_processing = true;
        self
    }

    /// Configure a source id.
    #[allow(unused)]
    pub(crate) fn with_source_id(mut self, source_id: SourceId) -> Self {
        self.source_id = source_id;
        self
    }

    /// Parse the prepared input.
    pub(crate) fn parse(self) -> compile::Result<Tree> {
        let source = match self.input {
            Input::Text(source) => {
                let mut lexer = Lexer::new(source, self.source_id, self.shebang);

                if self.without_processing {
                    lexer = lexer.without_processing();
                }

                Source::lexer(lexer)
            }
            Input::TokenStream(token_stream) => Source::token_stream(token_stream.iter()),
        };

        let mut p = Parser::new(source);
        self::grammar::root(&mut p)?;
        p.build()
    }
}
