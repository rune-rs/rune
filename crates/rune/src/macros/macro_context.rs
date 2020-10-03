//! Context for a macro.

use crate::ast;
use crate::macros::{Storage, ToTokens, TokenStream};
use runestick::{Source, Span};
use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

thread_local! {
    static MACRO_CONTEXT: RefCell<Option<MacroContext>> = RefCell::new(None);
}

/// Perform the given operation with the current macro context fetched from TLS.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub(crate) fn current_context<F, O>(f: F) -> O
where
    F: FnOnce(&MacroContext) -> O,
{
    MACRO_CONTEXT.with(|ctx| {
        let ctx = ctx
            .try_borrow()
            .expect("expected shared access to macro context");
        let ctx = ctx.as_ref().expect("missing macro context");
        f(ctx)
    })
}

/// Install the given context and call the provided function with the installed
/// context.
///
/// # Panics
///
/// This will panic if called while the current context is in use.
///
/// # Examples
///
/// ```rust
/// use rune::macros::{with_context, MacroContext};
/// let ctx = MacroContext::empty();
///
/// with_context(ctx, || {
///     rune::quote!(hello self);
/// });
/// ```
pub fn with_context<F, O>(new: MacroContext, f: F) -> O
where
    F: FnOnce() -> O,
{
    let old = MACRO_CONTEXT.with(|ctx| {
        let mut ctx = ctx
            .try_borrow_mut()
            .expect("expected exclusive access to macro context");

        ctx.replace(new)
    });

    let _guard = Guard(old);
    return f();

    struct Guard(Option<MacroContext>);

    impl Drop for Guard {
        fn drop(&mut self) {
            let old = self.0.take();

            MACRO_CONTEXT.with(|ctx| {
                let mut ctx = ctx
                    .try_borrow_mut()
                    .expect("expected exclusive access to macro context");

                *ctx = old;
            });
        }
    }
}

/// Context for a running macro.
pub struct MacroContext {
    /// The current source.
    pub(crate) source: Arc<Source>,
    /// Temporary recorded default span.
    pub(crate) span: Span,
    /// Storage used in macro context.
    pub(crate) storage: Storage,
}

impl MacroContext {
    /// Construct an empty macro context, primarily used for testing.
    pub fn empty() -> Self {
        Self {
            source: Arc::new(Source::default()),
            span: Span::empty(),
            storage: Storage::default(),
        }
    }

    /// Construct a new macro context.
    pub fn new(storage: Storage, source: Arc<Source>) -> Self {
        Self {
            source,
            span: Span::empty(),
            storage,
        }
    }

    /// Stringify the given token stream.
    pub(crate) fn stringify<'a, T>(&'a self, tokens: &T) -> Stringify<'_>
    where
        T: ToTokens,
    {
        let mut stream = TokenStream::empty();
        tokens.to_tokens(self, &mut stream);
        Stringify { ctx: self, stream }
    }

    /// Access the default span of the context.
    pub fn span(&self) -> Span {
        self.span
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
    pub(crate) fn lit<T>(&self, value: T) -> ast::Lit
    where
        T: IntoLit,
    {
        value.into_lit(self)
    }

    /// Construct a new identifier from the given string.
    pub(crate) fn ident(&self, ident: &str) -> ast::Ident {
        let id = self.storage.insert_str(ident);
        let source = ast::StringSource::Synthetic(id);

        ast::Ident {
            token: ast::Token {
                span: self.span,
                kind: ast::Kind::Ident(source),
            },
            source,
        }
    }

    /// Construct a new label from the given string. The string should be
    /// specified *without* the leading `'`, so `"foo"` instead of `"'foo"`.
    pub fn label(&self, label: &str) -> ast::Token {
        let id = self.storage.insert_str(label);

        ast::Token {
            span: self.span,
            kind: ast::Kind::Label(ast::StringSource::Synthetic(id)),
        }
    }

    /// Construct a new template string. This should be specified without the ``
    /// ` `` delimiters, so `"foo"` instead of ``"`foo`" ``.
    pub fn template_string(&self, string: &str) -> ast::Token {
        let id = self.storage.insert_str(string);

        ast::Token {
            span: self.span,
            kind: ast::Kind::LitTemplate(ast::LitStrSource::Synthetic(id)),
        }
    }
}

/// Helper trait used for things that can be converted into tokens.
pub trait IntoLit {
    /// Convert the current thing into a token.
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit;
}

impl IntoLit for i32 {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_number(self as i64);
        let source = ast::NumberSource::Synthetic(id);

        ast::Lit::Number(ast::LitNumber {
            token: ast::Token {
                kind: ast::Kind::LitNumber(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for i64 {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_number(self as i64);
        let source = ast::NumberSource::Synthetic(id);

        ast::Lit::Number(ast::LitNumber {
            token: ast::Token {
                kind: ast::Kind::LitNumber(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for f64 {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_number(self as i64);
        let source = ast::NumberSource::Synthetic(id);

        ast::Lit::Number(ast::LitNumber {
            token: ast::Token {
                kind: ast::Kind::LitNumber(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for char {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let source = ast::CopySource::Inline(self);

        ast::Lit::Char(ast::LitChar {
            token: ast::Token {
                kind: ast::Kind::LitChar(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for u8 {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let source = ast::CopySource::Inline(self);

        ast::Lit::Byte(ast::LitByte {
            token: ast::Token {
                kind: ast::Kind::LitByte(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for &str {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_str(self);
        let source = ast::LitStrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::LitStr(ast::LitStrSource::Synthetic(id)),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for &String {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        <&str>::into_lit(self, ctx)
    }
}

impl IntoLit for String {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_string(self);
        let source = ast::LitStrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::LitStr(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for &[u8] {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_byte_string(self);

        let source = ast::LitStrSource::Synthetic(id);

        ast::Lit::ByteStr(ast::LitByteStr {
            token: ast::Token {
                kind: ast::Kind::LitByteStr(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

macro_rules! impl_into_lit_byte_array {
    ($($n:literal),*) => {
        $(impl IntoLit for &[u8; $n] {
            fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
                <&[u8]>::into_lit(self, ctx)
            }
        })*
    };
}

impl_into_lit_byte_array! {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
}

pub struct Stringify<'a> {
    ctx: &'a MacroContext,
    stream: TokenStream,
}

impl<'a> fmt::Display for Stringify<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.stream.iter();
        let last = it.next_back();

        for token in it {
            token.token_fmt(self.ctx, f)?;
            write!(f, " ")?;
        }

        if let Some(last) = last {
            last.token_fmt(self.ctx, f)?;
        }

        Ok(())
    }
}
