//! Context for a macro.

use crate::ast;
use crate::ast::Span;
use crate::compile::{
    IrBudget, IrCompile, IrCompiler, IrError, IrErrorKind, IrEval, IrEvalOutcome, IrInterpreter,
    IrValue, ItemMeta, NoopCompileVisitor, UnitBuilder,
};
use crate::macros::{IntoLit, Storage, ToTokens, TokenStream};
use crate::parse::{Parse, ParseError, ParseErrorKind, Resolve, ResolveError};
use crate::query::{Query, Used};
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
        let gen = Gen::default();
        let mut consts = Consts::default();
        let mut storage = Storage::default();
        let mut sources = Sources::default();
        let mut visitor = NoopCompileVisitor::new();
        let mut inner = Default::default();

        let mut query = Query::new(
            &mut unit,
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
    /// ```rust
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
        T: IrCompile,
    {
        let mut ir_compiler = IrCompiler { q: self.q.borrow() };

        let output = ir_compiler.compile(target)?;

        let mut ir_interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            module: self.item.module.clone(),
            item: self.item.item.clone(),
            q: self.q.borrow(),
        };

        match output.eval(&mut ir_interpreter, Used::Used) {
            Ok(value) => Ok(value),
            Err(e) => match e {
                IrEvalOutcome::Error(error) => Err(error),
                IrEvalOutcome::NotConst(span) => Err(IrError::new(span, IrErrorKind::NotConst)),
                IrEvalOutcome::Break(span, _) => {
                    Err(IrError::new(span, IrErrorKind::BreakOutsideOfLoop))
                }
            },
        }
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
        item.resolve(self.q.storage, self.q.sources)
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

        crate::parse::parse_all(source.as_str(), id)
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

    /// Access storage associated with macro context.
    pub(crate) fn q(&self) -> &Query<'a> {
        &self.q
    }

    /// Access mutable storage associated with macro context.
    pub(crate) fn q_mut(&mut self) -> &mut Query<'a> {
        &mut self.q
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
