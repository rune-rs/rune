//! Context for a macro.

use crate::ast;
use crate::compiling::CompileError;
use crate::ir::{
    IrBudget, IrCompile, IrCompiler, IrErrorKind, IrEval, IrEvalOutcome, IrInterpreter,
};
use crate::macros::{Storage, ToTokens, TokenStream};
use crate::parsing::{ParseError, ResolveOwned};
use crate::query;
use crate::query::{QueryItem, Used};
use crate::shared::Consts;
use crate::Spanned;
use query::Query;
use runestick::{Source, Span};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    static MACRO_CONTEXT: RefCell<Option<MacroContext>> = RefCell::new(None);
}

/// Optionally get the span associated with the current context if it is
/// specified.
pub(crate) fn current_stream_span() -> Option<Span> {
    MACRO_CONTEXT.with(|ctx| Some(ctx.borrow().as_ref()?.stream_span()))
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
    /// The span of the macro call.
    pub(crate) macro_span: Span,
    /// Temporary recorded default span.
    pub(crate) stream_span: Span,
    /// The current source.
    pub(crate) source: Arc<Source>,
    /// Storage used in macro context.
    pub(crate) storage: Storage,
    /// Query engine.
    pub(crate) query: Query,
    /// The item where the macro is being evaluated.
    pub(crate) item: Rc<QueryItem>,
    /// Constants storage.
    pub(crate) consts: Consts,
}

impl MacroContext {
    /// Construct an empty macro context. Should only be used for testing.
    pub fn empty() -> Self {
        Self {
            macro_span: Span::empty(),
            stream_span: Span::empty(),
            source: Arc::new(Source::default()),
            storage: Storage::default(),
            query: Default::default(),
            item: Default::default(),
            consts: Default::default(),
        }
    }

    /// Resolve the given item into an owned variant.
    pub fn resolve_owned<T>(&self, item: T) -> Result<T::Owned, ParseError>
    where
        T: ResolveOwned,
    {
        item.resolve_owned(&self.storage, &self.source)
    }

    /// Evaluate the given ast as a constant expression.
    pub fn eval<T>(&self, target: &T) -> Result<<T::Output as IrEval>::Output, CompileError>
    where
        T: Spanned + IrCompile,
        T::Output: IrEval,
    {
        let mut ir_query = self.query.as_ir_query();

        let mut ir_compiler = IrCompiler {
            storage: self.storage.clone(),
            source: self.source.clone(),
            query: &mut *ir_query,
        };

        let output = ir_compiler.compile(target)?;

        let mut ir_interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            mod_item: self.item.mod_item.clone(),
            item: self.item.item.clone(),
            consts: self.consts.clone(),
            query: &mut *ir_query,
        };

        match ir_interpreter.eval(&output, Used::Used) {
            Ok(value) => Ok(value),
            Err(e) => match e {
                IrEvalOutcome::Error(error) => Err(CompileError::from(error)),
                IrEvalOutcome::NotConst(span) => {
                    Err(CompileError::new(span, Box::new(IrErrorKind::NotConst)))
                }
                IrEvalOutcome::Break(span, _) => Err(CompileError::new(
                    span,
                    Box::new(IrErrorKind::BreakOutsideOfLoop),
                )),
            },
        }
    }

    /// Stringify the given token stream.
    pub fn stringify<'a, T>(&'a self, tokens: &T) -> Stringify<'_>
    where
        T: ToTokens,
    {
        let mut stream = TokenStream::new();
        tokens.to_tokens(self, &mut stream);
        Stringify { ctx: self, stream }
    }

    /// Access span of the whole macro.
    pub fn macro_span(&self) -> Span {
        self.macro_span
    }

    /// Access the span of the stream being parsed.
    pub fn stream_span(&self) -> Span {
        self.stream_span
    }

    /// Access storage for the macro system.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Access the current source of the macro context.
    pub fn source(&self) -> &Source {
        &*self.source
    }
}

/// Helper trait used for things that can be converted into tokens.
pub trait IntoLit {
    /// Convert the current thing into a token.
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit;
}

impl<T> IntoLit for T
where
    ast::Number: From<T>,
{
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
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
    fn into_lit(self, span: Span, _: &Storage) -> ast::Lit {
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
    fn into_lit(self, span: Span, _: &Storage) -> ast::Lit {
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
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
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
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
        <&str>::into_lit(self, span, storage)
    }
}

impl IntoLit for String {
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
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
    fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
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
            fn into_lit(self, span: Span, storage: &Storage) -> ast::Lit {
                <&[u8]>::into_lit(self, span, storage)
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
