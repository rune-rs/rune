//! Context for a macro.

use crate::ast;
use crate::compiling::CompileError;
use crate::ir::{
    IrBudget, IrCompile, IrCompiler, IrError, IrErrorKind, IrEval, IrEvalOutcome, IrInterpreter,
};
use crate::macros::{Storage, ToTokens, TokenStream};
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

pub(crate) struct EvaluationContext {
    pub(crate) query: Query,
    pub(crate) item: Rc<QueryItem>,
    pub(crate) consts: Consts,
}

/// Context for a running macro.
pub struct MacroContext {
    /// The current source.
    pub(crate) source: Arc<Source>,
    /// Temporary recorded default span.
    pub(crate) span: Span,
    /// Storage used in macro context.
    pub(crate) storage: Storage,
    /// Query engine.
    pub(crate) eval_context: Option<EvaluationContext>,
}

impl MacroContext {
    /// Construct an empty macro context, primarily used for testing.
    pub fn empty() -> Self {
        Self {
            source: Arc::new(Source::default()),
            span: Span::empty(),
            storage: Storage::default(),
            eval_context: None,
        }
    }

    /// Evaluate the given ast as a constant expression.
    pub(crate) fn eval<T>(&self, target: &T) -> Result<<T::Output as IrEval>::Output, CompileError>
    where
        T: Spanned + IrCompile,
        T::Output: IrEval,
    {
        let eval_context = self
            .eval_context
            .as_ref()
            .ok_or_else(|| IrError::new(self.span, IrErrorKind::MissingMacroQuery))?;

        let mut ir_query = eval_context.query.as_ir_query();

        let mut ir_compiler = IrCompiler {
            storage: self.storage.clone(),
            source: self.source.clone(),
        };

        let output = ir_compiler.compile(target)?;

        let mut ir_interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            mod_item: eval_context.item.mod_item.clone(),
            item: eval_context.item.item.clone(),
            consts: eval_context.consts.clone(),
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
    pub(crate) fn stringify<'a, T>(&'a self, tokens: &T) -> Stringify<'_>
    where
        T: ToTokens,
    {
        let mut stream = TokenStream::new();
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
                kind: ast::Kind::Number(source),
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
                kind: ast::Kind::Number(source),
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
                kind: ast::Kind::Number(source),
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
                kind: ast::Kind::Char(source),
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
                kind: ast::Kind::Byte(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for &str {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_str(self);
        let source = ast::StrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Synthetic(id)),
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
        let source = ast::StrSource::Synthetic(id);

        ast::Lit::Str(ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::Str(source),
                span: ctx.span(),
            },
            source,
        })
    }
}

impl IntoLit for &[u8] {
    fn into_lit(self, ctx: &MacroContext) -> ast::Lit {
        let id = ctx.storage.insert_byte_string(self);

        let source = ast::StrSource::Synthetic(id);

        ast::Lit::ByteStr(ast::LitByteStr {
            token: ast::Token {
                kind: ast::Kind::ByteStr(source),
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
