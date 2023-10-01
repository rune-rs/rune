//! Context for a macro.

use core::fmt;

use crate::alloc;
use crate::ast;
use crate::ast::Span;
use crate::compile::ir;
use crate::compile::{self, ErrorKind, ItemMeta};
use crate::indexing::Indexer;
use crate::macros::{IntoLit, ToTokens, TokenStream};
use crate::parse::{Parse, Resolve};
use crate::{Source, SourceId};

cfg_std! {
    /// Construct an empty macro context which can be used for testing.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros;
    ///
    /// macros::test(|cx| {
    ///     let lit = cx.lit("hello world")?;
    ///     assert!(matches!(lit, ast::Lit::Str(..)));
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn test<F, O>(f: F) -> crate::support::Result<O>
    where
        F: FnOnce(&mut MacroContext<'_, '_, '_>) -> crate::support::Result<O>,
    {
        use crate::support::Context as _;
        use crate::compile::{Item, NoopCompileVisitor, NoopSourceLoader, Pool, Prelude, UnitBuilder};
        use crate::hir;
        use crate::indexing::{IndexItem, Items, Scopes};
        use crate::macros::Storage;
        use crate::query::Query;
        use crate::shared::{Consts, Gen};
        use crate::{Context, Diagnostics, Options, Sources};

        let mut unit = UnitBuilder::default();
        let prelude = Prelude::default();
        let gen = Gen::default();
        let const_arena = hir::Arena::new();
        let mut consts = Consts::default();
        let mut storage = Storage::default();
        let mut sources = Sources::default();
        let mut pool = Pool::new().context("Failed to allocate pool")?;
        let mut visitor = NoopCompileVisitor::new();
        let mut diagnostics = Diagnostics::default();
        let mut source_loader = NoopSourceLoader::default();
        let options = Options::default();
        let context = Context::default();
        let mut inner = Default::default();

        let mut query = Query::new(
            &mut unit,
            &prelude,
            &const_arena,
            &mut consts,
            &mut storage,
            &mut sources,
            &mut pool,
            &mut visitor,
            &mut diagnostics,
            &mut source_loader,
            &options,
            &gen,
            &context,
            &mut inner,
        );

        let root_id = gen.next();
        let source_id = SourceId::empty();

        let root_mod_id = query
            .insert_root_mod(root_id, source_id, Span::empty())
            .context("Failed to inserted root module")?;

        let item_meta = query
            .item_for(root_id)
            .context("Just inserted item meta does not exist")?;

        let mut idx = Indexer {
            q: query.borrow(),
            source_id,
            items: Items::new(Item::new(), root_id, &gen).context("Failed to construct items")?,
            scopes: Scopes::new().context("Failed to build indexer scopes")?,
            item: IndexItem::new(root_mod_id),
            nested_item: None,
            macro_depth: 0,
            root: None,
            queue: None,
            loaded: None,
        };

        let mut cx = MacroContext {
            macro_span: Span::empty(),
            input_span: Span::empty(),
            item_meta,
            idx: &mut idx,
        };

        f(&mut cx)
    }
}

/// Context for a running macro.
pub struct MacroContext<'a, 'b, 'arena> {
    /// Macro span of the full macro call.
    pub(crate) macro_span: Span,
    /// Macro span of the input.
    pub(crate) input_span: Span,
    /// The item where the macro is being evaluated.
    pub(crate) item_meta: ItemMeta,
    /// Indexer.
    pub(crate) idx: &'a mut Indexer<'b, 'arena>,
}

impl<'a, 'b, 'arena> MacroContext<'a, 'b, 'arena> {
    /// Evaluate the given target as a constant expression.
    ///
    /// # Panics
    ///
    /// This will panic if it's called outside of a macro context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rune::support::*;
    /// use rune::ast;
    /// use rune::macros::{self, quote};
    /// use rune::parse::{Parser};
    ///
    /// macros::test(|cx| {
    ///     let stream = quote!(1 + 2).into_token_stream(cx)?;
    ///
    ///     let mut p = Parser::from_token_stream(&stream, cx.input_span());
    ///     let expr = p.parse_all::<ast::Expr>()?;
    ///     let value = cx.eval(&expr)?;
    ///
    ///     let integer = value.into_integer::<u32>().context("Expected integer")?;
    ///     assert_eq!(3, integer);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn eval(&mut self, target: &ast::Expr) -> compile::Result<ir::Value> {
        target.eval(self)
    }

    /// Construct a new literal from within a macro context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros;
    ///
    /// macros::test(|cx| {
    ///     let lit = cx.lit("hello world")?;
    ///     assert!(matches!(lit, ast::Lit::Str(..)));
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn lit<T>(&mut self, lit: T) -> alloc::Result<ast::Lit>
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
    /// use rune::macros;
    ///
    /// macros::test(|cx| {
    ///     let lit = cx.ident("foo")?;
    ///     assert!(matches!(lit, ast::Ident { .. }));
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn ident(&mut self, ident: &str) -> alloc::Result<ast::Ident> {
        let span = self.macro_span();
        let id = self.idx.q.storage.insert_str(ident)?;
        let source = ast::LitSource::Synthetic(id);
        Ok(ast::Ident { span, source })
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
    /// use rune::macros;
    ///
    /// macros::test(|cx| {
    ///     let lit = cx.label("foo")?;
    ///     assert!(matches!(lit, ast::Label { .. }));
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn label(&mut self, label: &str) -> alloc::Result<ast::Label> {
        let span = self.macro_span();
        let id = self.idx.q.storage.insert_str(label)?;
        let source = ast::LitSource::Synthetic(id);
        Ok(ast::Label { span, source })
    }

    /// Stringify the token stream.
    pub fn stringify<T>(&mut self, tokens: &T) -> alloc::Result<Stringify<'_, 'a, 'b, 'arena>>
    where
        T: ToTokens,
    {
        let mut stream = TokenStream::new();
        tokens.to_tokens(self, &mut stream)?;
        Ok(Stringify { cx: self, stream })
    }

    /// Resolve the value of a token.
    pub fn resolve<'r, T>(&'r self, item: T) -> compile::Result<T::Output>
    where
        T: Resolve<'r>,
    {
        item.resolve(resolve_context!(self.idx.q))
    }

    /// Access a literal source as a string.
    pub(crate) fn literal_source(&self, source: ast::LitSource, span: Span) -> Option<&str> {
        match source {
            ast::LitSource::Text(source_id) => self.idx.q.sources.source(source_id, span),
            ast::LitSource::Synthetic(id) => self.idx.q.storage.get_string(id),
            ast::LitSource::BuiltIn(builtin) => Some(builtin.as_str()),
        }
    }

    /// Insert the given source so that it has a [SourceId] that can be used in
    /// combination with parsing functions such as
    /// [parse_source][MacroContext::parse_source].
    pub fn insert_source(&mut self, name: &str, source: &str) -> alloc::Result<SourceId> {
        self.idx.q.sources.insert(Source::new(name, source)?)
    }

    /// Parse the given input as the given type that implements
    /// [Parse][crate::parse::Parse].
    pub fn parse_source<T>(&self, id: SourceId) -> compile::Result<T>
    where
        T: Parse,
    {
        let source = self.idx.q.sources.get(id).ok_or_else(|| {
            compile::Error::new(Span::empty(), ErrorKind::MissingSourceId { source_id: id })
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
    pub fn input_span(&self) -> Span {
        self.input_span
    }
}

pub struct Stringify<'cx, 'a, 'b, 'arena> {
    cx: &'cx MacroContext<'a, 'b, 'arena>,
    stream: TokenStream,
}

impl fmt::Display for Stringify<'_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.stream.iter();
        let last = it.next_back();

        for token in it {
            token.token_fmt(self.cx, f)?;
            write!(f, " ")?;
        }

        if let Some(last) = last {
            last.token_fmt(self.cx, f)?;
        }

        Ok(())
    }
}
