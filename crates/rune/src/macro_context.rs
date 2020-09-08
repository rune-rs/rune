//! Context for a macro.

use crate::ast;
use crate::{Storage, TokenStream};
use runestick::{Source, Span};
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext {
    /// The current source.
    source: Arc<Source>,
    /// Temporary recorded default span.
    pub(crate) default_span: Span,
    /// End point of the span.
    pub(crate) end: Span,
    /// Storage used in macro context.
    pub(crate) storage: Storage,
}

impl MacroContext {
    /// Construct an empty macro context, primarily used for testing.
    pub fn empty() -> Self {
        Self {
            source: Arc::new(Source::default()),
            default_span: Span::empty(),
            end: Span::empty(),
            storage: Storage::default(),
        }
    }

    /// Construct a new macro context.
    pub fn new(storage: Storage, source: Arc<Source>) -> Self {
        Self {
            source,
            default_span: Span::empty(),
            end: Span::empty(),
            storage,
        }
    }

    /// Access the default span of the context.
    pub fn default_span(&self) -> Span {
        self.default_span
    }

    /// Construct a new token stream.
    pub fn token_stream(&self) -> TokenStream {
        TokenStream::new(Vec::new(), self.end)
    }

    /// Access storage for the macro system.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Access the current source of the macro context.
    pub fn source(&self) -> &Source {
        &*self.source
    }

    /// Construct a new literal token for values that can be converted into
    /// literals.
    pub fn lit<T>(&mut self, value: T) -> ast::Token
    where
        T: IntoLit,
    {
        value.into_lit(self)
    }

    /// Construct a new identifier from the given string.
    pub fn ident(&self, ident: &str) -> ast::Token {
        let id = self.storage.insert_string(ident);

        ast::Token {
            span: self.default_span,
            kind: ast::Kind::Ident(ast::StringSource::Synthetic(id)),
        }
    }

    /// Construct a new label from the given string. The string should be
    /// specified *without* the leading `'`, so `"foo"` instead of `"'foo"`.
    pub fn label(&self, label: &str) -> ast::Token {
        let id = self.storage.insert_string(label);

        ast::Token {
            span: self.default_span,
            kind: ast::Kind::Label(ast::StringSource::Synthetic(id)),
        }
    }

    /// Construct a new template string. This should be specified without the ``
    /// ` `` delimiters, so `"foo"` instead of ``"`foo`" ``.
    pub fn template_string(&self, string: &str) -> ast::Token {
        let id = self.storage.insert_string(string);

        ast::Token {
            span: self.default_span,
            kind: ast::Kind::LitTemplate(ast::LitStrSource::Synthetic(id)),
        }
    }
}

/// Helper trait used for things that can be converted into tokens.
pub trait IntoLit {
    /// Convert the current thing into a token.
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token;
}

impl IntoLit for i32 {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        let kind = ctx.storage.insert_number(self as i64);

        ast::Token {
            kind,
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for i64 {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        let kind = ctx.storage.insert_number(self);

        ast::Token {
            kind,
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for f64 {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        let kind = ctx.storage.insert_number(self);

        ast::Token {
            kind,
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for char {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        ast::Token {
            kind: ast::Kind::LitChar(ast::CopySource::Inline(self)),
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for u8 {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        ast::Token {
            kind: ast::Kind::LitByte(ast::CopySource::Inline(self)),
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for &str {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        let id = ctx.storage.insert_string(self);

        ast::Token {
            kind: ast::Kind::LitStr(ast::LitStrSource::Synthetic(id)),
            span: ctx.default_span(),
        }
    }
}

impl IntoLit for &String {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        <&str>::into_lit(self, ctx)
    }
}

impl IntoLit for &[u8] {
    fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
        let id = ctx.storage.insert_byte_string(self);

        ast::Token {
            kind: ast::Kind::LitByteStr(ast::LitByteStrSource::Synthetic(id)),
            span: ctx.default_span(),
        }
    }
}

macro_rules! impl_into_lit_byte_array {
    ($($n:literal),*) => {
        $(impl IntoLit for &[u8; $n] {
            fn into_lit(self, ctx: &mut MacroContext) -> ast::Token {
                <&[u8]>::into_lit(self, ctx)
            }
        })*
    };
}

impl_into_lit_byte_array! {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
}
