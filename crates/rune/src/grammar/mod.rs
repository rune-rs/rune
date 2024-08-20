//! This is the lossless and more relaxed parser for Rune.
//!
//! This produces a syntax tree that can be analysed using the provided methods.

mod classify;
pub(crate) use self::classify::{classify, NodeClass};

mod grammar;
pub(crate) use self::grammar::{object_key, ws};

mod parser;
use self::parser::Checkpoint;

mod tree;
use self::tree::{inner_token, InternalChildren};
pub(crate) use self::tree::{
    Ignore, MaybeNode, Node, NodeAt, NodeId, Remaining, Stream, StreamBuf, Tree,
};

mod flavor;
use self::flavor::Flavor;

use crate::ast::Kind;
use crate::macros::TokenStream;
use crate::parse::Lexer;
use crate::{compile, SourceId};

use self::parser::{Parser, Source};

/// Prepare parsing of text input.
pub(crate) fn text(source_id: SourceId, input: &str) -> Prepare<'_> {
    Prepare::new(Input::Text(source_id, input))
}

/// Prepare parsing of a token stream.
pub(crate) fn token_stream(token_stream: &TokenStream) -> Prepare<'_> {
    Prepare::new(Input::TokenStream(token_stream))
}

/// Prepare parsing of a flat tree.
pub(crate) fn node(tree: Node<'_>) -> Prepare<'_> {
    Prepare::new(Input::Node(tree))
}

enum Input<'a> {
    Text(SourceId, &'a str),
    TokenStream(&'a TokenStream),
    Node(Node<'a>),
}

/// A prepared parse.
pub(crate) struct Prepare<'a> {
    input: Input<'a>,
    without_processing: bool,
    include_whitespace: bool,
    shebang: bool,
}

impl<'a> Prepare<'a> {
    fn new(input: Input<'a>) -> Self {
        Self {
            input,
            without_processing: false,
            include_whitespace: false,
            shebang: true,
        }
    }

    /// Disable input processing.
    #[cfg(feature = "fmt")]
    pub(crate) fn without_processing(mut self) -> Self {
        self.without_processing = true;
        self
    }

    /// Configure whether to include whitespace.
    #[cfg(feature = "fmt")]
    pub(crate) fn include_whitespace(mut self) -> Self {
        self.include_whitespace = true;
        self
    }

    /// Parse the prepared input.
    pub(crate) fn root(self) -> compile::Result<Tree> {
        let mut p = self.into_parser();
        self::grammar::root(&mut p)?;
        p.build()
    }

    /// Parse a sequence of expressions.
    pub(crate) fn exprs(self, separator: Kind) -> compile::Result<Tree> {
        let mut p = self.into_parser();
        self::grammar::exprs(&mut p, separator)?;
        p.build()
    }

    /// Parse format arguments.
    pub(crate) fn format(self) -> compile::Result<Tree> {
        let mut p = self.into_parser();
        self::grammar::format(&mut p)?;
        p.build()
    }

    fn into_parser(self) -> Parser<'a> {
        let source = match self.input {
            Input::Text(source_id, source) => {
                let mut lexer = Lexer::new(source, source_id, self.shebang);

                if self.without_processing {
                    lexer = lexer.without_processing();
                }

                Source::lexer(lexer)
            }
            Input::TokenStream(token_stream) => Source::token_stream(token_stream.iter()),
            Input::Node(node) => Source::node(node),
        };

        let mut p = Parser::new(source);
        p.include_whitespace(self.include_whitespace);
        p
    }
}
