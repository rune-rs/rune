use proc_macro2 as p;

use crate::quote::{Builder, ToTokens};

pub(crate) const SCOPE: Punct = Punct::new("::");
pub(crate) const RUNE: Ident<'static> = Ident::new("rune");

pub(crate) const KIND: Ident<'static> = Ident::new("Kind");
pub(crate) const DELIMITER: Ident<'static> = Ident::new("Delimiter");
pub(crate) const TO_TOKENS_TYPE: Ident<'static> = Ident::new("ToTokens");
pub(crate) const TO_TOKENS: Ident<'static> = Ident::new("to_tokens");
pub(crate) const IDENT: Ident<'static> = Ident::new("Ident");
pub(crate) const LIT: Ident<'static> = Ident::new("Lit");
pub(crate) const NEW: Ident<'static> = Ident::new("new");

pub(crate) const RUNE_MACROS: RuneModule = RuneModule(Ident::new("macros"));
pub(crate) const RUNE_AST: RuneModule = RuneModule(Ident::new("ast"));

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuneModule(Ident<'static>);

impl ToTokens for RuneModule {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        self.0.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct KindVariant<T> {
    pub(crate) kind: Kind,
    pub(crate) variant: T,
}

impl<T> ToTokens for KindVariant<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        self.kind.to_tokens(stream, span);
        let mut builder = Builder::new();
        builder.push(self.variant);
        Group::new(p::Delimiter::Parenthesis, builder).to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ToTokensFn;

impl ToTokens for ToTokensFn {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE_MACROS.to_tokens(stream, span);
        TO_TOKENS_TYPE.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        TO_TOKENS.to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Ident<'a>(&'a str);

impl<'a> Ident<'a> {
    /// Construct an identifier.
    pub(crate) const fn new(name: &'a str) -> Ident<'a> {
        Ident(name)
    }
}

impl<'a> ToTokens for Ident<'a> {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        stream.extend(Some(p::TokenTree::Ident(p::Ident::new(self.0, span))))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Kind(&'static str);

impl Kind {
    pub(crate) const fn new(name: &'static str) -> Kind {
        Kind(name)
    }
}

impl ToTokens for Kind {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE_AST.to_tokens(stream, span);
        KIND.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        Ident::new(self.0).to_tokens(stream, span);
    }
}

pub(crate) struct Delimiter(&'static str);

impl Delimiter {
    pub(crate) const fn new(name: &'static str) -> Delimiter {
        Delimiter(name)
    }

    /// Convert from a proc macro.
    pub(crate) fn from_proc_macro(d: p::Delimiter) -> Option<Self> {
        match d {
            p::Delimiter::Parenthesis => Some(Delimiter::new("Parenthesis")),
            p::Delimiter::Brace => Some(Delimiter::new("Brace")),
            p::Delimiter::Bracket => Some(Delimiter::new("Bracket")),
            p::Delimiter::None => None,
        }
    }
}

impl ToTokens for Delimiter {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE_AST.to_tokens(stream, span);
        DELIMITER.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        Ident::new(self.0).to_tokens(stream, span);
    }
}

/// Construct a joined punctuation out of the given string.
pub(crate) struct Punct(&'static str, p::Spacing);

impl Punct {
    pub(crate) const fn new(s: &'static str) -> Punct {
        Punct(s, p::Spacing::Alone)
    }

    pub(crate) const fn joint(s: &'static str) -> Punct {
        Punct(s, p::Spacing::Joint)
    }
}

impl ToTokens for Punct {
    fn to_tokens(self, stream: &mut proc_macro2::TokenStream, span: p::Span) {
        let mut it = self.0.chars();
        let last = it.next_back();

        while let Some(c) = it.next() {
            let mut p = p::Punct::new(c, p::Spacing::Joint);
            p.set_span(span);
            stream.extend(Some(p::TokenTree::Punct(p)));
        }

        if let Some(c) = last {
            let mut p = p::Punct::new(c, self.1);
            p.set_span(span);
            stream.extend(Some(p::TokenTree::Punct(p)));
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Group(p::Delimiter, Builder);

impl Group {
    /// Construct a new group.
    pub(crate) const fn new(delimiter: p::Delimiter, content: Builder) -> Self {
        Group(delimiter, content)
    }
}

impl ToTokens for Group {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        let mut group = p::Group::new(self.0, self.1.into_stream());

        group.set_span(span);
        stream.extend(Some(p::TokenTree::Group(group)));
    }
}

/// An identifier constructor.
pub(crate) struct NewIdent<'a>(&'a str);

impl<'a> NewIdent<'a> {
    pub(crate) fn new(string: &'a str) -> Self {
        Self(string)
    }
}

impl<'a> ToTokens for NewIdent<'a> {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE_AST.to_tokens(stream, span);
        IDENT.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        NEW.to_tokens(stream, span);

        let args = p::TokenStream::from(p::TokenTree::Literal(p::Literal::string(self.0)));
        let mut group = p::Group::new(p::Delimiter::Parenthesis, args);

        group.set_span(span);
        stream.extend(Some(p::TokenTree::Group(group)));
    }
}

/// An identifier constructor.
pub(crate) struct NewLit(p::Literal);

impl NewLit {
    pub(crate) fn new(literal: p::Literal) -> Self {
        Self(literal)
    }
}

impl ToTokens for NewLit {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        RUNE_AST.to_tokens(stream, span);
        LIT.to_tokens(stream, span);
        SCOPE.to_tokens(stream, span);
        NEW.to_tokens(stream, span);

        let args = p::TokenStream::from(p::TokenTree::Literal(self.0));
        let mut group = p::Group::new(p::Delimiter::Parenthesis, args);

        group.set_span(span);
        stream.extend(Some(p::TokenTree::Group(group)));
    }
}
