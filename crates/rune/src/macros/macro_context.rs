//! Context for a macro.

use core::fmt;

use crate::alloc;
use crate::alloc::Vec;
use crate::ast;
use crate::ast::{OptionSpanned, Span};
use crate::compile::{self, ErrorKind, ItemMeta};
use crate::grammar::ws;
use crate::indexing::Indexer;
use crate::internal_macros::resolve_context;
use crate::macros::{IntoLit, ToTokens, TokenStream};
use crate::parse::{Parse, Parser, Resolve};
use crate::runtime::Value;
use crate::{Source, SourceId};

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
#[cfg(feature = "std")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
pub fn test<F, O>(f: F) -> crate::support::Result<O>
where
    F: FnOnce(&mut MacroContext<'_, '_, '_>) -> crate::support::Result<O>,
{
    use rust_alloc::rc::Rc;

    use crate::compile::{NoopCompileVisitor, NoopSourceLoader, Pool, Prelude, UnitBuilder};
    use crate::hir;
    use crate::indexing::{IndexItem, Items, Scopes};
    use crate::macros::Storage;
    use crate::query::Query;
    use crate::shared::{Consts, Gen};
    use crate::support::Context as _;
    use crate::{Context, Diagnostics, Item, Options, Sources};

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
    let options = Options::from_default_env()?;
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
        &[],
        &gen,
        &context,
        &mut inner,
    );

    let source_id = SourceId::empty();

    let (root_id, root_mod_id) = query
        .insert_root_mod(source_id, Span::empty())
        .context("Failed to inserted root module")?;

    let item_meta = query
        .item_for("root item", root_id)
        .context("Just inserted item meta does not exist")?;

    let tree = Rc::default();

    let mut idx = Indexer {
        q: query.borrow(),
        source_id,
        items: Items::new(Item::new()).context("Failed to construct items")?,
        scopes: Scopes::new().context("Failed to build indexer scopes")?,
        item: IndexItem::new(root_mod_id, root_id),
        nested_item: None,
        macro_depth: 0,
        root: None,
        queue: None,
        loaded: None,
        tree: &tree,
    };

    let mut cx = MacroContext {
        macro_span: Span::empty(),
        input_span: Span::empty(),
        item_meta,
        idx: &mut idx,
    };

    f(&mut cx)
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
    /// Construct a parser over a token stream, bounded by the compiler options
    /// this macro is being expanded under.
    ///
    /// The syntax tree this parser produces is walked by recursing over it, so
    /// how deep it is allowed to get is bounded by the `max-ast-depth` option.
    /// A parser built with [`Parser::from_token_stream`] instead uses that
    /// option's default, since it has no way of seeing what the compiler was
    /// configured with.
    ///
    /// A macro which only needs to know where each of its arguments ends does
    /// not need a tree at all - see [`MacroContext::exprs`], which splits an
    /// input without either recursing over it or holding it to that bound.
    ///
    /// `span` is the span to use if the stream is empty - typically
    /// [`MacroContext::input_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use rune::support::*;
    /// use rune::ast;
    /// use rune::macros::{self, quote};
    ///
    /// macros::test(|cx| {
    ///     let stream = quote!(1 + 2).into_token_stream(cx)?;
    ///
    ///     let mut p = cx.parser(&stream, cx.input_span());
    ///     let expr = p.parse_all::<ast::Expr>()?;
    ///     let value = cx.eval(&expr)?;
    ///
    ///     let integer = value.as_integer::<u32>().context("Expected integer")?;
    ///     assert_eq!(3, integer);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn parser<'s>(&self, token_stream: &'s TokenStream, span: Span) -> Parser<'s> {
        Parser::from_token_stream(token_stream, span)
            .with_max_depth(self.idx.q.options.max_ast_depth)
    }

    /// Split a token stream into the comma separated expressions it is made
    /// of, each one being the tokens it was written as.
    ///
    /// This is what a macro which takes a list of arguments uses to find where
    /// each one ends. The split is done by the same parser the compiler uses,
    /// which walks its input over an explicit stack, and each expression is
    /// handed back as tokens rather than as a syntax tree - so a macro built
    /// out of this neither recurses over its own input nor holds it to the
    /// much smaller `max-ast-depth` which bounds [`MacroContext::parser`].
    ///
    /// What comes back is where each argument ends rather than what it means:
    /// the tokens are handed on as they were written, and whatever they turn
    /// out to say is reported where the macro puts them, since that is where
    /// they are lowered.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rune::support::*;
    /// use rune::macros::{self, quote};
    ///
    /// macros::test(|cx| {
    ///     let stream = quote!("Hello {}", 1 + 2).into_token_stream(cx)?;
    ///
    ///     let exprs = cx.exprs(&stream)?;
    ///     assert_eq!(exprs.len(), 2);
    ///
    ///     let value = cx.eval_stream(&exprs[1])?;
    ///     assert_eq!(value.as_integer::<u32>()?, 3);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn exprs(&mut self, stream: &TokenStream) -> compile::Result<Vec<TokenStream>> {
        let span = self.stream_span(stream);

        let tree = crate::grammar::token_stream(stream)
            .max_nesting(self.idx.q.options.max_depth)
            .exprs(ast::Kind::Comma)?;

        let Some([root]) = tree.nodes() else {
            return Err(compile::Error::msg(span, "expected a single root"));
        };

        let mut exprs = Vec::new();

        // Whether the expression read most recently is still waiting for the
        // separator which ends it, which is what tells a missing separator
        // apart from a missing expression.
        let mut separated = true;

        for node in root.children() {
            if node.is_empty() {
                match node.kind() {
                    ws!() => {}
                    ast::Kind::Comma if !separated => {
                        separated = true;
                    }
                    _ => {
                        return Err(compile::Error::msg(node.span(), "expected an expression"));
                    }
                }

                continue;
            }

            if !separated {
                return Err(compile::Error::msg(node.span(), "expected `,`"));
            }

            let mut expr = TokenStream::new();

            for token in node.walk_tokens() {
                expr.push(token)?;
            }

            exprs.try_push(expr)?;
            separated = false;
        }

        Ok(exprs)
    }

    /// Evaluate the tokens of an expression as a constant.
    ///
    /// This is [`MacroContext::eval`] over the tokens an expression was
    /// written as, which is what a macro that split its input with
    /// [`MacroContext::exprs`] holds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rune::support::*;
    /// use rune::macros::{self, quote};
    ///
    /// macros::test(|cx| {
    ///     let stream = quote!(1 + 2).into_token_stream(cx)?;
    ///
    ///     let value = cx.eval_stream(&stream)?;
    ///     assert_eq!(value.as_integer::<u32>()?, 3);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn eval_stream(&mut self, stream: &TokenStream) -> compile::Result<Value> {
        let span = self.stream_span(stream);
        crate::compile::const_eval::eval_stream(self, stream, span)
    }

    /// The span of a token stream, which is the span of the input the macro
    /// was called with if the stream is empty.
    pub(crate) fn stream_span(&self, stream: &TokenStream) -> Span {
        stream.option_span().unwrap_or(self.input_span)
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
    ///     let integer = value.as_integer::<u32>().context("Expected integer")?;
    ///     assert_eq!(3, integer);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn eval(&mut self, target: &ast::Expr) -> compile::Result<Value> {
        crate::compile::const_eval::eval_ast(self, target)
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
