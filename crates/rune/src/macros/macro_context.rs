//! Context for a macro.

use crate::ast;
use crate::compile::{NoopCompileVisitor, UnitBuilder};
use crate::ir::IrError;
use crate::ir::{
    IrBudget, IrCompile, IrCompiler, IrErrorKind, IrEval, IrEvalOutcome, IrInterpreter,
};
use crate::macros::{Storage, ToTokens, TokenStream};
use crate::meta::CompileItem;
use crate::parse::{Parse, ParseError, ParseErrorKind, Resolve, ResolveError};
use crate::query::{Query, Used};
use crate::shared::Gen;
use crate::{Source, SourceId, Sources, Span, Spanned};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext<'a, 'q> {
    /// Macro span of the full macro call.
    pub(crate) macro_span: Span,
    /// Macro span of the stream.
    pub(crate) stream_span: Span,
    /// The item where the macro is being evaluated.
    pub(crate) item: Arc<CompileItem>,
    /// Accessible query required to run the IR interpreter and has access to
    /// storage.
    pub(crate) q: &'a mut Query<'q>,
}

impl<'a, 'q> MacroContext<'a, 'q> {
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
        F: FnOnce(&mut MacroContext<'_, '_>) -> O,
    {
        let mut unit = UnitBuilder::default();
        let gen = Gen::default();
        let mut sources = Sources::default();
        let mut query = Query::new(
            &mut unit,
            &mut sources,
            Rc::new(NoopCompileVisitor::new()),
            gen,
        );

        let mut ctx = MacroContext {
            macro_span: Span::empty(),
            stream_span: Span::empty(),
            item: Default::default(),
            q: &mut query,
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
    pub fn eval<T>(&mut self, target: &T) -> Result<<T::Output as IrEval>::Output, IrError>
    where
        T: Spanned + IrCompile,
        T::Output: IrEval,
    {
        let mut ir_compiler = IrCompiler { q: self.q };

        let output = ir_compiler.compile(target)?;

        let mut ir_interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            module: self.item.module.clone(),
            item: self.item.item.clone(),
            q: self.q,
        };

        match ir_interpreter.eval(&output, Used::Used) {
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

    /// Stringify the token stream.
    pub fn stringify<T>(&mut self, tokens: &T) -> Stringify<'_, 'a, 'q>
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
        item.resolve(&self.q.storage, self.q.sources)
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
    pub(crate) fn q(&self) -> &Query<'q> {
        self.q
    }

    /// Access mutable storage associated with macro context.
    pub(crate) fn q_mut(&mut self) -> &mut Query<'q> {
        self.q
    }
}

/// Helper trait used for things that can be converted into tokens.
pub trait IntoLit {
    /// Convert the current thing into a token.
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit;
}

impl<T> IntoLit for T
where
    ast::Number: From<T>,
{
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
        let id = storage.insert_number(self);
        let source = ast::NumberSource::Synthetic(id);

        ast::Lit::Number(ast::LitNumber {
            token: ast::Token {
                kind: ast::Kind::Number(source),
                span,
            },
            source,
        })
    }
}

impl IntoLit for char {
    fn into_lit(self, span: Span, _: &mut Storage) -> ast::Lit {
        let source = ast::CopySource::Inline(self);

        ast::Lit::Char(ast::LitChar {
            token: ast::Token {
                kind: ast::Kind::Char(source),
                span,
            },
            source,
        })
    }
}

impl IntoLit for u8 {
    fn into_lit(self, span: Span, _: &mut Storage) -> ast::Lit {
        let source = ast::CopySource::Inline(self);

        ast::Lit::Byte(ast::LitByte {
            token: ast::Token {
                kind: ast::Kind::Byte(source),
                span,
            },
            source,
        })
    }
}

impl IntoLit for &str {
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
        let id = storage.insert_str(self);
        let source = ast::StrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Synthetic(id)),
                span,
            },
            source,
        })
    }
}

impl IntoLit for &String {
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
        <&str>::into_lit(self, span, storage)
    }
}

impl IntoLit for String {
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
        let id = storage.insert_string(self);
        let source = ast::StrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::Str(source),
                span,
            },
            source,
        })
    }
}

impl IntoLit for &[u8] {
    fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
        let id = storage.insert_byte_string(self);
        let source = ast::StrSource::Synthetic(id);

        ast::Lit::ByteStr(ast::LitByteStr {
            token: ast::Token {
                kind: ast::Kind::ByteStr(source),
                span,
            },
            source,
        })
    }
}

macro_rules! impl_into_lit_byte_array {
    ($($n:literal),*) => {
        $(impl IntoLit for &[u8; $n] {
            fn into_lit(self, span: Span, storage: &mut Storage) -> ast::Lit {
                <&[u8]>::into_lit(self, span, storage)
            }
        })*
    };
}

impl_into_lit_byte_array! {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
}

pub struct Stringify<'ctx, 'a, 'q> {
    ctx: &'ctx MacroContext<'a, 'q>,
    stream: TokenStream,
}

impl fmt::Display for Stringify<'_, '_, '_> {
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
