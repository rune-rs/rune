//! Context for a macro.

use crate::ast;
use crate::ast::Span;
use crate::compile::{
    IrCompiler, IrError, IrEval, IrEvalContext, IrValue, ItemMeta, NoopCompileVisitor, Prelude,
    UnitBuilder,
};
use crate::macros::{IntoLit, Storage, ToTokens, TokenStream};
use crate::parse::{Parse, ParseError, ParseErrorKind, Resolve, ResolveError};
use crate::query::Query;
use crate::shared::{Consts, Gen};
use crate::{Source, SourceId, Sources};
use std::fmt;
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext<'a> {
    /// Macro span of the full macro call.
    pub(crate) macro_span: Span,
    /// Macro span of the stream.
    pub(crate) stream_span: Span,
    /// The item where the macro is being evaluated.
    pub(crate) item: Arc<ItemMeta>,
    /// Accessible query required to run the IR interpreter and has access to
    /// storage.
    pub(crate) q: Query<'a>,
}

impl<'a> MacroContext<'a> {
    /// Construct an empty macro context which can be used for testing.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::macros::MacroContext;
    ///
    /// MacroContext::test(|ctx| ());
    /// ```
    pub fn test<F, O>(f: F) -> O
    where
        F: FnOnce(&mut MacroContext<'_>) -> O,
    {
        let mut unit = UnitBuilder::default();
        let prelude = Prelude::default();
        let gen = Gen::default();
        let mut consts = Consts::default();
        let mut storage = Storage::default();
        let mut sources = Sources::default();
        let mut visitor = NoopCompileVisitor::new();
        let mut inner = Default::default();

        let mut query = Query::new(
            &mut unit,
            &prelude,
            &mut consts,
            &mut storage,
            &mut sources,
            &mut visitor,
            &gen,
            &mut inner,
        );

        let mut ctx = MacroContext {
            macro_span: Span::empty(),
            stream_span: Span::empty(),
            item: Default::default(),
            q: query.borrow(),
        };

        f(&mut ctx)
    }

    /// Evaluate the given target as a constant expression.
    ///
    /// # Panics
    ///
    /// This will panic if it's called outside of a macro context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros::{MacroContext, quote};
    /// use rune::parse::{Parser};
    ///
    /// // Note: should only be used for testing.
    /// MacroContext::test(|ctx| {
    ///     let stream = quote!(1 + 2).into_token_stream(ctx);
    ///
    ///     let mut p = Parser::from_token_stream(&stream, ctx.stream_span());
    ///     let expr = p.parse_all::<ast::Expr>().unwrap();
    ///     let value = ctx.eval(&expr).unwrap();
    ///
    ///     assert_eq!(3, value.into_integer::<u32>().unwrap());
    /// });
    /// ```
    pub fn eval<T>(&mut self, target: &T) -> Result<IrValue, IrError>
    where
        T: IrEval,
    {
        let mut ctx = IrEvalContext {
            c: IrCompiler {
                source_id: self.item.location.source_id,
                q: self.q.borrow(),
            },
            item: &self.item,
        };

        target.eval(&mut ctx)
    }

    /// Construct a new literal from within a macro context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros::MacroContext;
    ///
    /// MacroContext::test(|ctx| {
    ///     let lit = ctx.lit("hello world");
    ///     assert!(matches!(lit, ast::Lit::Str(..)))
    /// });
    /// ```
    pub fn lit<T>(&mut self, lit: T) -> ast::Lit
    where
        T: IntoLit,
    {
        T::into_lit(lit, self)
    }

    /// Construct a new identifier from the given string from inside of a macro
    /// context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros::MacroContext;
    ///
    /// MacroContext::test(|ctx| {
    ///     let lit = ctx.ident("foo");
    ///     assert!(matches!(lit, ast::Ident { .. }))
    /// });
    /// ```
    pub fn ident(&mut self, ident: &str) -> ast::Ident {
        let span = self.macro_span();
        let id = self.q.storage.insert_str(ident);
        let source = ast::LitSource::Synthetic(id);
        ast::Ident { span, source }
    }

    /// Construct a new label from the given string. The string should be
    /// specified *without* the leading `'`, so `"foo"` instead of `"'foo"`.
    ///
    /// This constructor does not panic when called outside of a macro context
    /// but requires access to a `span` and `storage`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros::MacroContext;
    ///
    /// MacroContext::test(|ctx| {
    ///     let lit = ctx.label("foo");
    ///     assert!(matches!(lit, ast::Label { .. }))
    /// });
    /// ```
    pub fn label(&mut self, label: &str) -> ast::Label {
        let span = self.macro_span();
        let id = self.q.storage.insert_str(label);
        let source = ast::LitSource::Synthetic(id);
        ast::Label { span, source }
    }

    /// Stringify the token stream.
    pub fn stringify<T>(&mut self, tokens: &T) -> Stringify<'_, 'a>
    where
        T: ToTokens,
    {
        let mut stream = TokenStream::new();
        tokens.to_tokens(self, &mut stream);
        Stringify { ctx: self, stream }
    }

    /// Resolve the value of a token.
    pub fn resolve<'r, T>(&'r self, item: T) -> Result<T::Output, ResolveError>
    where
        T: Resolve<'r>,
    {
        item.resolve(resolve_context!(self.q))
    }

    /// Access a literal source as a string.
    pub(crate) fn literal_source(&self, source: ast::LitSource, span: Span) -> Option<&str> {
        match source {
            ast::LitSource::Text(source_id) => self.q.sources.source(source_id, span),
            ast::LitSource::Synthetic(id) => self.q.storage.get_string(id),
            ast::LitSource::BuiltIn(builtin) => Some(builtin.as_str()),
        }
    }

    /// Insert the given source so that it has a [SourceId] that can be used in
    /// combination with parsing functions such as
    /// [parse_source][MacroContext::parse_source].
    pub fn insert_source(&mut self, name: &str, source: &str) -> SourceId {
        self.q.sources.insert(Source::new(name, source))
    }

    /// Parse the given input as the given type that implements
    /// [Parse][crate::parse::Parse].
    pub fn parse_source<T>(&self, id: SourceId) -> Result<T, ParseError>
    where
        T: Parse,
    {
        let source = self.q.sources.get(id).ok_or_else(|| {
            ParseError::new(
                Span::empty(),
                ParseErrorKind::MissingSourceId { source_id: id },
            )
        })?;

        crate::parse::parse_all(source.as_str(), id, false)
    }

    /// The span of the macro call including the name of the macro.
    ///
    /// If the macro call was `stringify!(a + b)` this would refer to the whole
    /// macro call.
    pub fn macro_span(&self) -> Span {
        self.macro_span
    }

    /// The span of the macro stream (the argument).
    ///
    /// If the macro call was `stringify!(a + b)` this would refer to `a + b`.
    pub fn stream_span(&self) -> Span {
        self.stream_span
    }
}

pub struct Stringify<'ctx, 'a> {
    ctx: &'ctx MacroContext<'a>,
    stream: TokenStream,
}

impl fmt::Display for Stringify<'_, '_> {
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
