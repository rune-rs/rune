use crate::alloc::clone;
use crate::ast;
use crate::compile;
use crate::macros;
use crate::parse;
use core::fmt;

use crate as rune;

// This file has been generated from `assets/tokens.yaml`
// DO NOT modify by hand!

/// The `abstract` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Abstract {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Abstract {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Abstract {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Abstract {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Abstract => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Abstract,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Abstract => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("abstract")
    }
}

impl parse::Parse for Abstract {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Abstract => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Abstract)),
        }
    }
}

impl parse::Peek for Abstract {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Abstract)
    }
}

impl macros::ToTokens for Abstract {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Abstract,
        })
    }
}

/// The `alignof` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct AlignOf {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for AlignOf {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AlignOf {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for AlignOf {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::AlignOf => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::AlignOf,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::AlignOf => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("alignof")
    }
}

impl parse::Parse for AlignOf {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AlignOf => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::AlignOf)),
        }
    }
}

impl parse::Peek for AlignOf {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AlignOf)
    }
}

impl macros::ToTokens for AlignOf {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::AlignOf,
        })
    }
}

/// `&`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Amp {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Amp {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Amp {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Amp {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Amp => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Amp,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Amp => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("&")
    }
}

impl parse::Parse for Amp {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Amp => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Amp)),
        }
    }
}

impl parse::Peek for Amp {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Amp)
    }
}

impl macros::ToTokens for Amp {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Amp,
        })
    }
}

/// `&&`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct AmpAmp {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for AmpAmp {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AmpAmp {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for AmpAmp {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::AmpAmp => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::AmpAmp,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::AmpAmp => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("&&")
    }
}

impl parse::Parse for AmpAmp {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AmpAmp => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::AmpAmp)),
        }
    }
}

impl parse::Peek for AmpAmp {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AmpAmp)
    }
}

impl macros::ToTokens for AmpAmp {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::AmpAmp,
        })
    }
}

/// `&=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct AmpEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for AmpEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AmpEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for AmpEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::AmpEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::AmpEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::AmpEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("&=")
    }
}

impl parse::Parse for AmpEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AmpEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::AmpEq)),
        }
    }
}

impl parse::Peek for AmpEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AmpEq)
    }
}

impl macros::ToTokens for AmpEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::AmpEq,
        })
    }
}

/// `->`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Arrow {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Arrow {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Arrow {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Arrow {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Arrow => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Arrow,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Arrow => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("->")
    }
}

impl parse::Parse for Arrow {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Arrow => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Arrow)),
        }
    }
}

impl parse::Peek for Arrow {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Arrow)
    }
}

impl macros::ToTokens for Arrow {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Arrow,
        })
    }
}

/// The `as` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct As {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for As {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for As {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for As {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::As => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::As,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::As => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("as")
    }
}

impl parse::Parse for As {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::As => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::As)),
        }
    }
}

impl parse::Peek for As {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::As)
    }
}

impl macros::ToTokens for As {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::As,
        })
    }
}

/// The `async` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Async {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Async {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Async {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Async {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Async => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Async,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Async => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("async")
    }
}

impl parse::Parse for Async {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Async => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Async)),
        }
    }
}

impl parse::Peek for Async {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Async)
    }
}

impl macros::ToTokens for Async {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Async,
        })
    }
}

/// `@`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct At {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for At {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for At {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for At {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::At => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::At,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::At => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("@")
    }
}

impl parse::Parse for At {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::At => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::At)),
        }
    }
}

impl parse::Peek for At {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::At)
    }
}

impl macros::ToTokens for At {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::At,
        })
    }
}

/// The `await` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Await {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Await {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Await {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Await {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Await => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Await,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Await => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("await")
    }
}

impl parse::Parse for Await {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Await => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Await)),
        }
    }
}

impl parse::Peek for Await {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Await)
    }
}

impl macros::ToTokens for Await {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Await,
        })
    }
}

/// `!`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Bang {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Bang {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Bang {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Bang {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Bang => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Bang,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Bang => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("!")
    }
}

impl parse::Parse for Bang {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Bang => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Bang)),
        }
    }
}

impl parse::Peek for Bang {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Bang)
    }
}

impl macros::ToTokens for Bang {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Bang,
        })
    }
}

/// `!=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct BangEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for BangEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for BangEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for BangEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::BangEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::BangEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::BangEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("!=")
    }
}

impl parse::Parse for BangEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::BangEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::BangEq)),
        }
    }
}

impl parse::Peek for BangEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::BangEq)
    }
}

impl macros::ToTokens for BangEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::BangEq,
        })
    }
}

/// The `become` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Become {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Become {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Become {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Become {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Become => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Become,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Become => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("become")
    }
}

impl parse::Parse for Become {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Become => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Become)),
        }
    }
}

impl parse::Peek for Become {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Become)
    }
}

impl macros::ToTokens for Become {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Become,
        })
    }
}

/// The `break` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Break {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Break {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Break {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Break {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Break => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Break,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Break => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("break")
    }
}

impl parse::Parse for Break {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Break => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Break)),
        }
    }
}

impl parse::Peek for Break {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Break)
    }
}

impl macros::ToTokens for Break {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Break,
        })
    }
}

/// `^`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Caret {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Caret {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Caret {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Caret {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Caret => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Caret,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Caret => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("^")
    }
}

impl parse::Parse for Caret {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Caret => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Caret)),
        }
    }
}

impl parse::Peek for Caret {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Caret)
    }
}

impl macros::ToTokens for Caret {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Caret,
        })
    }
}

/// `^=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct CaretEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for CaretEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for CaretEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for CaretEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::CaretEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::CaretEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::CaretEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("^=")
    }
}

impl parse::Parse for CaretEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::CaretEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::CaretEq)),
        }
    }
}

impl parse::Peek for CaretEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::CaretEq)
    }
}

impl macros::ToTokens for CaretEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::CaretEq,
        })
    }
}

/// `:`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Colon {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Colon {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Colon {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Colon {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Colon => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Colon,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Colon => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(":")
    }
}

impl parse::Parse for Colon {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Colon => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Colon)),
        }
    }
}

impl parse::Peek for Colon {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Colon)
    }
}

impl macros::ToTokens for Colon {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Colon,
        })
    }
}

/// `::`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct ColonColon {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for ColonColon {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for ColonColon {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for ColonColon {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::ColonColon => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::ColonColon,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::ColonColon => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("::")
    }
}

impl parse::Parse for ColonColon {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::ColonColon => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::ColonColon)),
        }
    }
}

impl parse::Peek for ColonColon {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::ColonColon)
    }
}

impl macros::ToTokens for ColonColon {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::ColonColon,
        })
    }
}

/// `,`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Comma {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Comma {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Comma {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Comma {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Comma => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Comma,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Comma => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(",")
    }
}

impl parse::Parse for Comma {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Comma => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Comma)),
        }
    }
}

impl parse::Peek for Comma {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Comma)
    }
}

impl macros::ToTokens for Comma {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Comma,
        })
    }
}

/// The `const` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Const {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Const {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Const {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Const {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Const => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Const,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Const => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("const")
    }
}

impl parse::Parse for Const {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Const => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Const)),
        }
    }
}

impl parse::Peek for Const {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Const)
    }
}

impl macros::ToTokens for Const {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Const,
        })
    }
}

/// The `continue` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Continue {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Continue {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Continue {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Continue {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Continue => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Continue,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Continue => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("continue")
    }
}

impl parse::Parse for Continue {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Continue => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Continue)),
        }
    }
}

impl parse::Peek for Continue {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Continue)
    }
}

impl macros::ToTokens for Continue {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Continue,
        })
    }
}

/// The `crate` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Crate {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Crate {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Crate {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Crate {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Crate => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Crate,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Crate => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("crate")
    }
}

impl parse::Parse for Crate {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Crate => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Crate)),
        }
    }
}

impl parse::Peek for Crate {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Crate)
    }
}

impl macros::ToTokens for Crate {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Crate,
        })
    }
}

/// `-`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Dash {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Dash {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dash {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Dash {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Dash => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Dash,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Dash => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("-")
    }
}

impl parse::Parse for Dash {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dash => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Dash)),
        }
    }
}

impl parse::Peek for Dash {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dash)
    }
}

impl macros::ToTokens for Dash {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Dash,
        })
    }
}

/// `-=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct DashEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for DashEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DashEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for DashEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::DashEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::DashEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::DashEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("-=")
    }
}

impl parse::Parse for DashEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DashEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::DashEq)),
        }
    }
}

impl parse::Peek for DashEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DashEq)
    }
}

impl macros::ToTokens for DashEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::DashEq,
        })
    }
}

/// The `default` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Default {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Default {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Default {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Default {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Default => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Default,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Default => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("default")
    }
}

impl parse::Parse for Default {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Default => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Default)),
        }
    }
}

impl parse::Peek for Default {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Default)
    }
}

impl macros::ToTokens for Default {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Default,
        })
    }
}

/// `/`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Div {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Div {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Div {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Div {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Div => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Div,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Div => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("/")
    }
}

impl parse::Parse for Div {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Div => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Div)),
        }
    }
}

impl parse::Peek for Div {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Div)
    }
}

impl macros::ToTokens for Div {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Div,
        })
    }
}

/// The `do` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Do {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Do {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Do {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Do {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Do => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Do,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Do => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("do")
    }
}

impl parse::Parse for Do {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Do => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Do)),
        }
    }
}

impl parse::Peek for Do {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Do)
    }
}

impl macros::ToTokens for Do {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Do,
        })
    }
}

/// `$`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Dollar {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Dollar {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dollar {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Dollar {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Dollar => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Dollar,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Dollar => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("$")
    }
}

impl parse::Parse for Dollar {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dollar => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Dollar)),
        }
    }
}

impl parse::Peek for Dollar {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dollar)
    }
}

impl macros::ToTokens for Dollar {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Dollar,
        })
    }
}

/// `.`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Dot {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Dot {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dot {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Dot {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Dot => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Dot,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Dot => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(".")
    }
}

impl parse::Parse for Dot {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dot => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Dot)),
        }
    }
}

impl parse::Peek for Dot {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dot)
    }
}

impl macros::ToTokens for Dot {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Dot,
        })
    }
}

/// `..`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct DotDot {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for DotDot {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DotDot {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for DotDot {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::DotDot => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::DotDot,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::DotDot => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("..")
    }
}

impl parse::Parse for DotDot {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DotDot => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::DotDot)),
        }
    }
}

impl parse::Peek for DotDot {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DotDot)
    }
}

impl macros::ToTokens for DotDot {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::DotDot,
        })
    }
}

/// `..=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct DotDotEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for DotDotEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DotDotEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for DotDotEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::DotDotEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::DotDotEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::DotDotEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("..=")
    }
}

impl parse::Parse for DotDotEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DotDotEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::DotDotEq)),
        }
    }
}

impl parse::Peek for DotDotEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DotDotEq)
    }
}

impl macros::ToTokens for DotDotEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::DotDotEq,
        })
    }
}

/// The `else` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Else {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Else {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Else {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Else {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Else => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Else,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Else => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("else")
    }
}

impl parse::Parse for Else {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Else => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Else)),
        }
    }
}

impl parse::Peek for Else {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Else)
    }
}

impl macros::ToTokens for Else {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Else,
        })
    }
}

/// The `enum` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Enum {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Enum {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Enum {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Enum {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Enum => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Enum,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Enum => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("enum")
    }
}

impl parse::Parse for Enum {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Enum => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Enum)),
        }
    }
}

impl parse::Peek for Enum {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Enum)
    }
}

impl macros::ToTokens for Enum {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Enum,
        })
    }
}

/// `=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Eq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Eq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Eq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Eq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Eq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Eq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Eq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("=")
    }
}

impl parse::Parse for Eq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Eq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Eq)),
        }
    }
}

impl parse::Peek for Eq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Eq)
    }
}

impl macros::ToTokens for Eq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Eq,
        })
    }
}

/// `==`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct EqEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for EqEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for EqEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for EqEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::EqEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::EqEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::EqEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("==")
    }
}

impl parse::Parse for EqEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::EqEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::EqEq)),
        }
    }
}

impl parse::Peek for EqEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::EqEq)
    }
}

impl macros::ToTokens for EqEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::EqEq,
        })
    }
}

/// The `extern` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Extern {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Extern {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Extern {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Extern {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Extern => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Extern,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Extern => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("extern")
    }
}

impl parse::Parse for Extern {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Extern => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Extern)),
        }
    }
}

impl parse::Peek for Extern {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Extern)
    }
}

impl macros::ToTokens for Extern {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Extern,
        })
    }
}

/// The `false` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct False {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for False {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for False {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for False {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::False => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::False,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::False => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("false")
    }
}

impl parse::Parse for False {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::False => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::False)),
        }
    }
}

impl parse::Peek for False {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::False)
    }
}

impl macros::ToTokens for False {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::False,
        })
    }
}

/// The `final` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Final {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Final {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Final {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Final {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Final => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Final,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Final => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("final")
    }
}

impl parse::Parse for Final {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Final => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Final)),
        }
    }
}

impl parse::Peek for Final {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Final)
    }
}

impl macros::ToTokens for Final {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Final,
        })
    }
}

/// The `fn` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Fn {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Fn {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Fn {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Fn {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Fn => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Fn,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Fn => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("fn")
    }
}

impl parse::Parse for Fn {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Fn => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Fn)),
        }
    }
}

impl parse::Peek for Fn {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Fn)
    }
}

impl macros::ToTokens for Fn {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Fn,
        })
    }
}

/// The `for` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct For {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for For {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for For {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for For {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::For => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::For,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::For => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("for")
    }
}

impl parse::Parse for For {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::For => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::For)),
        }
    }
}

impl parse::Peek for For {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::For)
    }
}

impl macros::ToTokens for For {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::For,
        })
    }
}

/// `>`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Gt {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Gt {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Gt {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Gt {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Gt => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Gt,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Gt => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(">")
    }
}

impl parse::Parse for Gt {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Gt => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Gt)),
        }
    }
}

impl parse::Peek for Gt {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Gt)
    }
}

impl macros::ToTokens for Gt {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Gt,
        })
    }
}

/// `>=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct GtEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for GtEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for GtEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::GtEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::GtEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::GtEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(">=")
    }
}

impl parse::Parse for GtEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::GtEq)),
        }
    }
}

impl parse::Peek for GtEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtEq)
    }
}

impl macros::ToTokens for GtEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::GtEq,
        })
    }
}

/// `>>`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct GtGt {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for GtGt {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtGt {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for GtGt {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::GtGt => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::GtGt,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::GtGt => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(">>")
    }
}

impl parse::Parse for GtGt {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtGt => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::GtGt)),
        }
    }
}

impl parse::Peek for GtGt {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtGt)
    }
}

impl macros::ToTokens for GtGt {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::GtGt,
        })
    }
}

/// `>>=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct GtGtEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for GtGtEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtGtEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for GtGtEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::GtGtEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::GtGtEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::GtGtEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(">>=")
    }
}

impl parse::Parse for GtGtEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtGtEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::GtGtEq)),
        }
    }
}

impl parse::Peek for GtGtEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtGtEq)
    }
}

impl macros::ToTokens for GtGtEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::GtGtEq,
        })
    }
}

/// The `if` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct If {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for If {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for If {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for If {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::If => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::If,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::If => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("if")
    }
}

impl parse::Parse for If {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::If => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::If)),
        }
    }
}

impl parse::Peek for If {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::If)
    }
}

impl macros::ToTokens for If {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::If,
        })
    }
}

/// The `impl` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Impl {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Impl {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Impl {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Impl {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Impl => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Impl,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Impl => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("impl")
    }
}

impl parse::Parse for Impl {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Impl => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Impl)),
        }
    }
}

impl parse::Peek for Impl {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Impl)
    }
}

impl macros::ToTokens for Impl {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Impl,
        })
    }
}

/// The `in` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct In {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for In {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for In {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for In {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::In => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::In,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::In => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("in")
    }
}

impl parse::Parse for In {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::In => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::In)),
        }
    }
}

impl parse::Peek for In {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::In)
    }
}

impl macros::ToTokens for In {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::In,
        })
    }
}

/// The `is` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Is {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Is {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Is {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Is {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Is => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Is,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Is => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("is")
    }
}

impl parse::Parse for Is {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Is => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Is)),
        }
    }
}

impl parse::Peek for Is {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Is)
    }
}

impl macros::ToTokens for Is {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Is,
        })
    }
}

/// The `let` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Let {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Let {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Let {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Let {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Let => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Let,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Let => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("let")
    }
}

impl parse::Parse for Let {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Let => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Let)),
        }
    }
}

impl parse::Peek for Let {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Let)
    }
}

impl macros::ToTokens for Let {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Let,
        })
    }
}

/// The `loop` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Loop {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Loop {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Loop {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Loop {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Loop => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Loop,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Loop => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("loop")
    }
}

impl parse::Parse for Loop {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Loop => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Loop)),
        }
    }
}

impl parse::Peek for Loop {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Loop)
    }
}

impl macros::ToTokens for Loop {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Loop,
        })
    }
}

/// `<`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Lt {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Lt {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Lt {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Lt {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Lt => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Lt,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Lt => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("<")
    }
}

impl parse::Parse for Lt {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Lt => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Lt)),
        }
    }
}

impl parse::Peek for Lt {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Lt)
    }
}

impl macros::ToTokens for Lt {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Lt,
        })
    }
}

/// `<=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LtEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for LtEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for LtEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::LtEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::LtEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::LtEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("<=")
    }
}

impl parse::Parse for LtEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::LtEq)),
        }
    }
}

impl parse::Peek for LtEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtEq)
    }
}

impl macros::ToTokens for LtEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::LtEq,
        })
    }
}

/// `<<`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LtLt {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for LtLt {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtLt {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for LtLt {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::LtLt => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::LtLt,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::LtLt => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("<<")
    }
}

impl parse::Parse for LtLt {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtLt => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::LtLt)),
        }
    }
}

impl parse::Peek for LtLt {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtLt)
    }
}

impl macros::ToTokens for LtLt {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::LtLt,
        })
    }
}

/// `<<=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LtLtEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for LtLtEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtLtEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for LtLtEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::LtLtEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::LtLtEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::LtLtEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("<<=")
    }
}

impl parse::Parse for LtLtEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtLtEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::LtLtEq)),
        }
    }
}

impl parse::Peek for LtLtEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtLtEq)
    }
}

impl macros::ToTokens for LtLtEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::LtLtEq,
        })
    }
}

/// The `macro` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Macro {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Macro {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Macro {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Macro {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Macro => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Macro,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Macro => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("macro")
    }
}

impl parse::Parse for Macro {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Macro => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Macro)),
        }
    }
}

impl parse::Peek for Macro {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Macro)
    }
}

impl macros::ToTokens for Macro {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Macro,
        })
    }
}

/// The `match` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Match {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Match {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Match {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Match {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Match => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Match,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Match => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("match")
    }
}

impl parse::Parse for Match {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Match => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Match)),
        }
    }
}

impl parse::Peek for Match {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Match)
    }
}

impl macros::ToTokens for Match {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Match,
        })
    }
}

/// The `mod` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Mod {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Mod {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Mod {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Mod {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Mod => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Mod,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Mod => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("mod")
    }
}

impl parse::Parse for Mod {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Mod => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Mod)),
        }
    }
}

impl parse::Peek for Mod {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Mod)
    }
}

impl macros::ToTokens for Mod {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Mod,
        })
    }
}

/// The `move` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Move {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Move {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Move {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Move {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Move => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Move,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Move => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("move")
    }
}

impl parse::Parse for Move {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Move => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Move)),
        }
    }
}

impl parse::Peek for Move {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Move)
    }
}

impl macros::ToTokens for Move {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Move,
        })
    }
}

/// The `mut` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Mut {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Mut {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Mut {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Mut {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Mut => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Mut,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Mut => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("mut")
    }
}

impl parse::Parse for Mut {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Mut => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Mut)),
        }
    }
}

impl parse::Peek for Mut {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Mut)
    }
}

impl macros::ToTokens for Mut {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Mut,
        })
    }
}

/// The `not` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Not {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Not {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Not {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Not {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Not => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Not,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Not => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("not")
    }
}

impl parse::Parse for Not {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Not => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Not)),
        }
    }
}

impl parse::Peek for Not {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Not)
    }
}

impl macros::ToTokens for Not {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Not,
        })
    }
}

/// The `offsetof` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct OffsetOf {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for OffsetOf {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for OffsetOf {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for OffsetOf {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::OffsetOf => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::OffsetOf,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::OffsetOf => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("offsetof")
    }
}

impl parse::Parse for OffsetOf {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::OffsetOf => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::OffsetOf)),
        }
    }
}

impl parse::Peek for OffsetOf {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::OffsetOf)
    }
}

impl macros::ToTokens for OffsetOf {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::OffsetOf,
        })
    }
}

/// The `override` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Override {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Override {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Override {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Override {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Override => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Override,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Override => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("override")
    }
}

impl parse::Parse for Override {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Override => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Override)),
        }
    }
}

impl parse::Peek for Override {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Override)
    }
}

impl macros::ToTokens for Override {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Override,
        })
    }
}

/// `%`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Perc {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Perc {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Perc {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Perc {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Perc => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Perc,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Perc => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("%")
    }
}

impl parse::Parse for Perc {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Perc => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Perc)),
        }
    }
}

impl parse::Peek for Perc {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Perc)
    }
}

impl macros::ToTokens for Perc {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Perc,
        })
    }
}

/// `%=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct PercEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for PercEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PercEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for PercEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::PercEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::PercEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::PercEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("%=")
    }
}

impl parse::Parse for PercEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PercEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::PercEq)),
        }
    }
}

impl parse::Peek for PercEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PercEq)
    }
}

impl macros::ToTokens for PercEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::PercEq,
        })
    }
}

/// `|`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Pipe {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Pipe {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pipe {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Pipe {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Pipe => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Pipe,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Pipe => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("|")
    }
}

impl parse::Parse for Pipe {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pipe => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Pipe)),
        }
    }
}

impl parse::Peek for Pipe {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pipe)
    }
}

impl macros::ToTokens for Pipe {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Pipe,
        })
    }
}

/// |=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct PipeEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for PipeEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PipeEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for PipeEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::PipeEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::PipeEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::PipeEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("|=")
    }
}

impl parse::Parse for PipeEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PipeEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::PipeEq)),
        }
    }
}

impl parse::Peek for PipeEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PipeEq)
    }
}

impl macros::ToTokens for PipeEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::PipeEq,
        })
    }
}

/// `||`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct PipePipe {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for PipePipe {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PipePipe {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for PipePipe {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::PipePipe => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::PipePipe,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::PipePipe => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("||")
    }
}

impl parse::Parse for PipePipe {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PipePipe => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::PipePipe)),
        }
    }
}

impl parse::Peek for PipePipe {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PipePipe)
    }
}

impl macros::ToTokens for PipePipe {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::PipePipe,
        })
    }
}

/// `+`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Plus {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Plus {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Plus {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Plus {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Plus => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Plus,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Plus => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("+")
    }
}

impl parse::Parse for Plus {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Plus => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Plus)),
        }
    }
}

impl parse::Peek for Plus {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Plus)
    }
}

impl macros::ToTokens for Plus {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Plus,
        })
    }
}

/// `+=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct PlusEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for PlusEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PlusEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for PlusEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::PlusEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::PlusEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::PlusEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("+=")
    }
}

impl parse::Parse for PlusEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PlusEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::PlusEq)),
        }
    }
}

impl parse::Peek for PlusEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PlusEq)
    }
}

impl macros::ToTokens for PlusEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::PlusEq,
        })
    }
}

/// `#`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Pound {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Pound {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pound {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Pound {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Pound => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Pound,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Pound => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("#")
    }
}

impl parse::Parse for Pound {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pound => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Pound)),
        }
    }
}

impl parse::Peek for Pound {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pound)
    }
}

impl macros::ToTokens for Pound {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Pound,
        })
    }
}

/// The `priv` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Priv {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Priv {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Priv {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Priv {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Priv => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Priv,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Priv => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("priv")
    }
}

impl parse::Parse for Priv {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Priv => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Priv)),
        }
    }
}

impl parse::Peek for Priv {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Priv)
    }
}

impl macros::ToTokens for Priv {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Priv,
        })
    }
}

/// The `proc` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Proc {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Proc {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Proc {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Proc {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Proc => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Proc,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Proc => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("proc")
    }
}

impl parse::Parse for Proc {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Proc => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Proc)),
        }
    }
}

impl parse::Peek for Proc {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Proc)
    }
}

impl macros::ToTokens for Proc {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Proc,
        })
    }
}

/// The `pub` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Pub {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Pub {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pub {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Pub {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Pub => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Pub,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Pub => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("pub")
    }
}

impl parse::Parse for Pub {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pub => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Pub)),
        }
    }
}

impl parse::Peek for Pub {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pub)
    }
}

impl macros::ToTokens for Pub {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Pub,
        })
    }
}

/// The `pure` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Pure {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Pure {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pure {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Pure {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Pure => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Pure,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Pure => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("pure")
    }
}

impl parse::Parse for Pure {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pure => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Pure)),
        }
    }
}

impl parse::Peek for Pure {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pure)
    }
}

impl macros::ToTokens for Pure {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Pure,
        })
    }
}

/// `?`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct QuestionMark {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for QuestionMark {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for QuestionMark {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for QuestionMark {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::QuestionMark => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::QuestionMark,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::QuestionMark => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("?")
    }
}

impl parse::Parse for QuestionMark {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::QuestionMark => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::QuestionMark)),
        }
    }
}

impl parse::Peek for QuestionMark {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::QuestionMark)
    }
}

impl macros::ToTokens for QuestionMark {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::QuestionMark,
        })
    }
}

/// The `ref` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Ref {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Ref {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Ref {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Ref {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Ref => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Ref,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Ref => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("ref")
    }
}

impl parse::Parse for Ref {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Ref => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Ref)),
        }
    }
}

impl parse::Peek for Ref {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Ref)
    }
}

impl macros::ToTokens for Ref {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Ref,
        })
    }
}

/// The `return` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Return {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Return {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Return {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Return {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Return => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Return,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Return => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("return")
    }
}

impl parse::Parse for Return {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Return => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Return)),
        }
    }
}

impl parse::Peek for Return {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Return)
    }
}

impl macros::ToTokens for Return {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Return,
        })
    }
}

/// `=>`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Rocket {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Rocket {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Rocket {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Rocket {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Rocket => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Rocket,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Rocket => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("=>")
    }
}

impl parse::Parse for Rocket {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Rocket => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Rocket)),
        }
    }
}

impl parse::Peek for Rocket {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Rocket)
    }
}

impl macros::ToTokens for Rocket {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Rocket,
        })
    }
}

/// The `select` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Select {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Select {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Select {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Select {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Select => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Select,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Select => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("select")
    }
}

impl parse::Parse for Select {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Select => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Select)),
        }
    }
}

impl parse::Peek for Select {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Select)
    }
}

impl macros::ToTokens for Select {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Select,
        })
    }
}

/// The `Self` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct SelfType {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for SelfType {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SelfType {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for SelfType {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::SelfType => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::SelfType,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::SelfType => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("Self")
    }
}

impl parse::Parse for SelfType {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SelfType => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::SelfType)),
        }
    }
}

impl parse::Peek for SelfType {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SelfType)
    }
}

impl macros::ToTokens for SelfType {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::SelfType,
        })
    }
}

/// The `self` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct SelfValue {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for SelfValue {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SelfValue {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for SelfValue {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::SelfValue => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::SelfValue,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::SelfValue => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("self")
    }
}

impl parse::Parse for SelfValue {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SelfValue => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::SelfValue)),
        }
    }
}

impl parse::Peek for SelfValue {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SelfValue)
    }
}

impl macros::ToTokens for SelfValue {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::SelfValue,
        })
    }
}

/// `;`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct SemiColon {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for SemiColon {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SemiColon {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for SemiColon {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::SemiColon => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::SemiColon,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::SemiColon => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation(";")
    }
}

impl parse::Parse for SemiColon {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SemiColon => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::SemiColon)),
        }
    }
}

impl parse::Peek for SemiColon {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SemiColon)
    }
}

impl macros::ToTokens for SemiColon {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::SemiColon,
        })
    }
}

/// The `sizeof` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct SizeOf {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for SizeOf {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SizeOf {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for SizeOf {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::SizeOf => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::SizeOf,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::SizeOf => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("sizeof")
    }
}

impl parse::Parse for SizeOf {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SizeOf => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::SizeOf)),
        }
    }
}

impl parse::Peek for SizeOf {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SizeOf)
    }
}

impl macros::ToTokens for SizeOf {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::SizeOf,
        })
    }
}

/// `/=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct SlashEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for SlashEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SlashEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for SlashEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::SlashEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::SlashEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::SlashEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("/=")
    }
}

impl parse::Parse for SlashEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SlashEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::SlashEq)),
        }
    }
}

impl parse::Peek for SlashEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SlashEq)
    }
}

impl macros::ToTokens for SlashEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::SlashEq,
        })
    }
}

/// `*`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Star {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Star {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Star {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Star {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Star => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Star,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Star => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("*")
    }
}

impl parse::Parse for Star {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Star => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Star)),
        }
    }
}

impl parse::Peek for Star {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Star)
    }
}

impl macros::ToTokens for Star {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Star,
        })
    }
}

/// `*=`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct StarEq {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for StarEq {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for StarEq {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for StarEq {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::StarEq => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::StarEq,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::StarEq => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("*=")
    }
}

impl parse::Parse for StarEq {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::StarEq => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::StarEq)),
        }
    }
}

impl parse::Peek for StarEq {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::StarEq)
    }
}

impl macros::ToTokens for StarEq {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::StarEq,
        })
    }
}

/// The `static` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Static {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Static {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Static {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Static {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Static => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Static,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Static => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("static")
    }
}

impl parse::Parse for Static {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Static => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Static)),
        }
    }
}

impl parse::Peek for Static {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Static)
    }
}

impl macros::ToTokens for Static {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Static,
        })
    }
}

/// The `struct` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Struct {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Struct {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Struct {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Struct {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Struct => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Struct,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Struct => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("struct")
    }
}

impl parse::Parse for Struct {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Struct => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Struct)),
        }
    }
}

impl parse::Peek for Struct {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Struct)
    }
}

impl macros::ToTokens for Struct {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Struct,
        })
    }
}

/// The `super` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Super {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Super {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Super {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Super {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Super => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Super,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Super => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("super")
    }
}

impl parse::Parse for Super {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Super => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Super)),
        }
    }
}

impl parse::Peek for Super {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Super)
    }
}

impl macros::ToTokens for Super {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Super,
        })
    }
}

/// `~`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Tilde {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Tilde {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Tilde {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Tilde {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Tilde => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Tilde,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Tilde => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("~")
    }
}

impl parse::Parse for Tilde {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Tilde => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Tilde)),
        }
    }
}

impl parse::Peek for Tilde {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Tilde)
    }
}

impl macros::ToTokens for Tilde {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Tilde,
        })
    }
}

/// The `true` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct True {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for True {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for True {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for True {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::True => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::True,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::True => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("true")
    }
}

impl parse::Parse for True {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::True => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::True)),
        }
    }
}

impl parse::Peek for True {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::True)
    }
}

impl macros::ToTokens for True {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::True,
        })
    }
}

/// The `typeof` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct TypeOf {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for TypeOf {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for TypeOf {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for TypeOf {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::TypeOf => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::TypeOf,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::TypeOf => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("typeof")
    }
}

impl parse::Parse for TypeOf {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::TypeOf => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::TypeOf)),
        }
    }
}

impl parse::Peek for TypeOf {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::TypeOf)
    }
}

impl macros::ToTokens for TypeOf {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::TypeOf,
        })
    }
}

/// `_`.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Underscore {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Underscore {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Underscore {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Underscore {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Underscore => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Underscore,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Underscore => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Punctuation("_")
    }
}

impl parse::Parse for Underscore {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Underscore => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Underscore)),
        }
    }
}

impl parse::Peek for Underscore {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Underscore)
    }
}

impl macros::ToTokens for Underscore {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Underscore,
        })
    }
}

/// The `unsafe` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Unsafe {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Unsafe {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Unsafe {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Unsafe {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Unsafe => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Unsafe,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Unsafe => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("unsafe")
    }
}

impl parse::Parse for Unsafe {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Unsafe => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Unsafe)),
        }
    }
}

impl parse::Peek for Unsafe {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Unsafe)
    }
}

impl macros::ToTokens for Unsafe {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Unsafe,
        })
    }
}

/// The `use` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Use {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Use {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Use {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Use {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Use => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Use,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Use => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("use")
    }
}

impl parse::Parse for Use {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Use => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Use)),
        }
    }
}

impl parse::Peek for Use {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Use)
    }
}

impl macros::ToTokens for Use {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Use,
        })
    }
}

/// The `virtual` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Virtual {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Virtual {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Virtual {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Virtual {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Virtual => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Virtual,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Virtual => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("virtual")
    }
}

impl parse::Parse for Virtual {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Virtual => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Virtual)),
        }
    }
}

impl parse::Peek for Virtual {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Virtual)
    }
}

impl macros::ToTokens for Virtual {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Virtual,
        })
    }
}

/// The `while` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct While {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for While {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for While {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for While {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::While => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::While,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::While => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("while")
    }
}

impl parse::Parse for While {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::While => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::While)),
        }
    }
}

impl parse::Peek for While {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::While)
    }
}

impl macros::ToTokens for While {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::While,
        })
    }
}

/// The `yield` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Yield {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Yield {
    #[inline]
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Yield {
    #[inline]
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
    }
}

impl ast::ToAst for Yield {
    fn to_ast(span: ast::Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            ast::Kind::Yield => Ok(Self { span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                ast::Kind::Yield,
            )),
        }
    }

    fn matches(kind: &ast::Kind) -> bool {
        match kind {
            ast::Kind::Yield => true,
            _ => false,
        }
    }

    #[inline]
    fn into_expectation() -> parse::Expectation {
        parse::Expectation::Keyword("yield")
    }
}

impl parse::Parse for Yield {
    fn parse(p: &mut parse::Parser<'_>) -> compile::Result<Self> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Yield => Ok(Self { span: token.span }),
            _ => Err(compile::Error::expected(token, ast::Kind::Yield)),
        }
    }
}

impl parse::Peek for Yield {
    #[inline]
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Yield)
    }
}

impl macros::ToTokens for Yield {
    fn to_tokens(
        &self,
        _: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Yield,
        })
    }
}

/// Helper macro to reference a specific token.
#[macro_export]
macro_rules! T {
    ('(') => {
        $crate::ast::OpenParen
    };
    (')') => {
        $crate::ast::CloseParen
    };
    ('[') => {
        $crate::ast::OpenBracket
    };
    (']') => {
        $crate::ast::CloseBracket
    };
    ('{') => {
        $crate::ast::OpenBrace
    };
    ('}') => {
        $crate::ast::CloseBrace
    };
    (is not) => {
        $crate::ast::IsNot
    };
    (abstract) => {
        $crate::ast::Abstract
    };
    (alignof) => {
        $crate::ast::AlignOf
    };
    (as) => {
        $crate::ast::As
    };
    (async) => {
        $crate::ast::Async
    };
    (await) => {
        $crate::ast::Await
    };
    (become) => {
        $crate::ast::Become
    };
    (break) => {
        $crate::ast::Break
    };
    (const) => {
        $crate::ast::Const
    };
    (continue) => {
        $crate::ast::Continue
    };
    (crate) => {
        $crate::ast::Crate
    };
    (default) => {
        $crate::ast::Default
    };
    (do) => {
        $crate::ast::Do
    };
    (else) => {
        $crate::ast::Else
    };
    (enum) => {
        $crate::ast::Enum
    };
    (extern) => {
        $crate::ast::Extern
    };
    (false) => {
        $crate::ast::False
    };
    (final) => {
        $crate::ast::Final
    };
    (fn) => {
        $crate::ast::Fn
    };
    (for) => {
        $crate::ast::For
    };
    (if) => {
        $crate::ast::If
    };
    (impl) => {
        $crate::ast::Impl
    };
    (in) => {
        $crate::ast::In
    };
    (is) => {
        $crate::ast::Is
    };
    (let) => {
        $crate::ast::Let
    };
    (loop) => {
        $crate::ast::Loop
    };
    (macro) => {
        $crate::ast::Macro
    };
    (match) => {
        $crate::ast::Match
    };
    (mod) => {
        $crate::ast::Mod
    };
    (move) => {
        $crate::ast::Move
    };
    (mut) => {
        $crate::ast::Mut
    };
    (not) => {
        $crate::ast::Not
    };
    (offsetof) => {
        $crate::ast::OffsetOf
    };
    (override) => {
        $crate::ast::Override
    };
    (priv) => {
        $crate::ast::Priv
    };
    (proc) => {
        $crate::ast::Proc
    };
    (pub) => {
        $crate::ast::Pub
    };
    (pure) => {
        $crate::ast::Pure
    };
    (ref) => {
        $crate::ast::Ref
    };
    (return) => {
        $crate::ast::Return
    };
    (select) => {
        $crate::ast::Select
    };
    (Self) => {
        $crate::ast::SelfType
    };
    (self) => {
        $crate::ast::SelfValue
    };
    (sizeof) => {
        $crate::ast::SizeOf
    };
    (static) => {
        $crate::ast::Static
    };
    (struct) => {
        $crate::ast::Struct
    };
    (super) => {
        $crate::ast::Super
    };
    (true) => {
        $crate::ast::True
    };
    (typeof) => {
        $crate::ast::TypeOf
    };
    (unsafe) => {
        $crate::ast::Unsafe
    };
    (use) => {
        $crate::ast::Use
    };
    (virtual) => {
        $crate::ast::Virtual
    };
    (while) => {
        $crate::ast::While
    };
    (yield) => {
        $crate::ast::Yield
    };
    (&) => {
        $crate::ast::Amp
    };
    (&&) => {
        $crate::ast::AmpAmp
    };
    (&=) => {
        $crate::ast::AmpEq
    };
    (->) => {
        $crate::ast::Arrow
    };
    (@) => {
        $crate::ast::At
    };
    (!) => {
        $crate::ast::Bang
    };
    (!=) => {
        $crate::ast::BangEq
    };
    (^) => {
        $crate::ast::Caret
    };
    (^=) => {
        $crate::ast::CaretEq
    };
    (:) => {
        $crate::ast::Colon
    };
    (::) => {
        $crate::ast::ColonColon
    };
    (,) => {
        $crate::ast::Comma
    };
    (-) => {
        $crate::ast::Dash
    };
    (-=) => {
        $crate::ast::DashEq
    };
    (/) => {
        $crate::ast::Div
    };
    ($) => {
        $crate::ast::Dollar
    };
    (.) => {
        $crate::ast::Dot
    };
    (..) => {
        $crate::ast::DotDot
    };
    (..=) => {
        $crate::ast::DotDotEq
    };
    (=) => {
        $crate::ast::Eq
    };
    (==) => {
        $crate::ast::EqEq
    };
    (>) => {
        $crate::ast::Gt
    };
    (>=) => {
        $crate::ast::GtEq
    };
    (>>) => {
        $crate::ast::GtGt
    };
    (>>=) => {
        $crate::ast::GtGtEq
    };
    (<) => {
        $crate::ast::Lt
    };
    (<=) => {
        $crate::ast::LtEq
    };
    (<<) => {
        $crate::ast::LtLt
    };
    (<<=) => {
        $crate::ast::LtLtEq
    };
    (%) => {
        $crate::ast::Perc
    };
    (%=) => {
        $crate::ast::PercEq
    };
    (|) => {
        $crate::ast::Pipe
    };
    (|=) => {
        $crate::ast::PipeEq
    };
    (||) => {
        $crate::ast::PipePipe
    };
    (+) => {
        $crate::ast::Plus
    };
    (+=) => {
        $crate::ast::PlusEq
    };
    (#) => {
        $crate::ast::Pound
    };
    (?) => {
        $crate::ast::QuestionMark
    };
    (=>) => {
        $crate::ast::Rocket
    };
    (;) => {
        $crate::ast::SemiColon
    };
    (/=) => {
        $crate::ast::SlashEq
    };
    (*) => {
        $crate::ast::Star
    };
    (*=) => {
        $crate::ast::StarEq
    };
    (~) => {
        $crate::ast::Tilde
    };
    (_) => {
        $crate::ast::Underscore
    };
}

/// Helper macro to reference a specific token kind, or short sequence of kinds.
#[macro_export]
macro_rules! K {
    (#!($($tt:tt)*)) => { $crate::ast::Kind::Shebang($($tt)*) };
    (ident) => { $crate::ast::Kind::Ident(..) };
    (ident ($($tt:tt)*)) => { $crate::ast::Kind::Ident($($tt)*) };
    ('label) => { $crate::ast::Kind::Label(..) };
    ('label ($($tt:tt)*)) => { $crate::ast::Kind::Label($($tt)*) };
    (str) => { $crate::ast::Kind::Str(..) };
    (str ($($tt:tt)*)) => { $crate::ast::Kind::Str($($tt)*) };
    (bytestr) => { $crate::ast::Kind::ByteStr(..) };
    (bytestr ($($tt:tt)*)) => { $crate::ast::Kind::ByteStr($($tt)*) };
    (char) => { $crate::ast::Kind::Char(..) };
    (char ($($tt:tt)*)) => { $crate::ast::Kind::Char($($tt)*) };
    (byte) => { $crate::ast::Kind::Byte(..) };
    (byte ($($tt:tt)*)) => { $crate::ast::Kind::Byte($($tt)*) };
    (number) => { $crate::ast::Kind::Number(..) };
    (number ($($tt:tt)*)) => { $crate::ast::Kind::Number($($tt)*) };
    ('(') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Parenthesis) };
    (')') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Parenthesis) };
    ('[') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Bracket) };
    (']') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Bracket) };
    ('{') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Brace) };
    ('}') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Brace) };
    (abstract) => { $crate::ast::Kind::Abstract };
    (alignof) => { $crate::ast::Kind::AlignOf };
    (as) => { $crate::ast::Kind::As };
    (async) => { $crate::ast::Kind::Async };
    (await) => { $crate::ast::Kind::Await };
    (become) => { $crate::ast::Kind::Become };
    (break) => { $crate::ast::Kind::Break };
    (const) => { $crate::ast::Kind::Const };
    (continue) => { $crate::ast::Kind::Continue };
    (crate) => { $crate::ast::Kind::Crate };
    (default) => { $crate::ast::Kind::Default };
    (do) => { $crate::ast::Kind::Do };
    (else) => { $crate::ast::Kind::Else };
    (enum) => { $crate::ast::Kind::Enum };
    (extern) => { $crate::ast::Kind::Extern };
    (false) => { $crate::ast::Kind::False };
    (final) => { $crate::ast::Kind::Final };
    (fn) => { $crate::ast::Kind::Fn };
    (for) => { $crate::ast::Kind::For };
    (if) => { $crate::ast::Kind::If };
    (impl) => { $crate::ast::Kind::Impl };
    (in) => { $crate::ast::Kind::In };
    (is) => { $crate::ast::Kind::Is };
    (let) => { $crate::ast::Kind::Let };
    (loop) => { $crate::ast::Kind::Loop };
    (macro) => { $crate::ast::Kind::Macro };
    (match) => { $crate::ast::Kind::Match };
    (mod) => { $crate::ast::Kind::Mod };
    (move) => { $crate::ast::Kind::Move };
    (mut) => { $crate::ast::Kind::Mut };
    (not) => { $crate::ast::Kind::Not };
    (offsetof) => { $crate::ast::Kind::OffsetOf };
    (override) => { $crate::ast::Kind::Override };
    (priv) => { $crate::ast::Kind::Priv };
    (proc) => { $crate::ast::Kind::Proc };
    (pub) => { $crate::ast::Kind::Pub };
    (pure) => { $crate::ast::Kind::Pure };
    (ref) => { $crate::ast::Kind::Ref };
    (return) => { $crate::ast::Kind::Return };
    (select) => { $crate::ast::Kind::Select };
    (Self) => { $crate::ast::Kind::SelfType };
    (self) => { $crate::ast::Kind::SelfValue };
    (sizeof) => { $crate::ast::Kind::SizeOf };
    (static) => { $crate::ast::Kind::Static };
    (struct) => { $crate::ast::Kind::Struct };
    (super) => { $crate::ast::Kind::Super };
    (true) => { $crate::ast::Kind::True };
    (typeof) => { $crate::ast::Kind::TypeOf };
    (unsafe) => { $crate::ast::Kind::Unsafe };
    (use) => { $crate::ast::Kind::Use };
    (virtual) => { $crate::ast::Kind::Virtual };
    (while) => { $crate::ast::Kind::While };
    (yield) => { $crate::ast::Kind::Yield };
    (&) => { $crate::ast::Kind::Amp };
    (&&) => { $crate::ast::Kind::AmpAmp };
    (&=) => { $crate::ast::Kind::AmpEq };
    (->) => { $crate::ast::Kind::Arrow };
    (@) => { $crate::ast::Kind::At };
    (!) => { $crate::ast::Kind::Bang };
    (!=) => { $crate::ast::Kind::BangEq };
    (^) => { $crate::ast::Kind::Caret };
    (^=) => { $crate::ast::Kind::CaretEq };
    (:) => { $crate::ast::Kind::Colon };
    (::) => { $crate::ast::Kind::ColonColon };
    (,) => { $crate::ast::Kind::Comma };
    (-) => { $crate::ast::Kind::Dash };
    (-=) => { $crate::ast::Kind::DashEq };
    (/) => { $crate::ast::Kind::Div };
    ($) => { $crate::ast::Kind::Dollar };
    (.) => { $crate::ast::Kind::Dot };
    (..) => { $crate::ast::Kind::DotDot };
    (..=) => { $crate::ast::Kind::DotDotEq };
    (=) => { $crate::ast::Kind::Eq };
    (==) => { $crate::ast::Kind::EqEq };
    (>) => { $crate::ast::Kind::Gt };
    (>=) => { $crate::ast::Kind::GtEq };
    (>>) => { $crate::ast::Kind::GtGt };
    (>>=) => { $crate::ast::Kind::GtGtEq };
    (<) => { $crate::ast::Kind::Lt };
    (<=) => { $crate::ast::Kind::LtEq };
    (<<) => { $crate::ast::Kind::LtLt };
    (<<=) => { $crate::ast::Kind::LtLtEq };
    (%) => { $crate::ast::Kind::Perc };
    (%=) => { $crate::ast::Kind::PercEq };
    (|) => { $crate::ast::Kind::Pipe };
    (|=) => { $crate::ast::Kind::PipeEq };
    (||) => { $crate::ast::Kind::PipePipe };
    (+) => { $crate::ast::Kind::Plus };
    (+=) => { $crate::ast::Kind::PlusEq };
    (#) => { $crate::ast::Kind::Pound };
    (?) => { $crate::ast::Kind::QuestionMark };
    (=>) => { $crate::ast::Kind::Rocket };
    (;) => { $crate::ast::Kind::SemiColon };
    (/=) => { $crate::ast::Kind::SlashEq };
    (*) => { $crate::ast::Kind::Star };
    (*=) => { $crate::ast::Kind::StarEq };
    (~) => { $crate::ast::Kind::Tilde };
    (_) => { $crate::ast::Kind::Underscore };
}

/// The kind of the token.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// En end-of-file marker.
    Eof,
    /// A single-line comment.
    Comment,
    /// A multiline comment where the boolean indicates if it's been terminated correctly.
    MultilineComment(bool),
    /// En error marker.
    Error,
    /// The special initial line of a file shebang.
    Shebang(ast::LitSource),
    /// A close delimiter: `)`, `}`, or `]`.
    Close(ast::Delimiter),
    /// An open delimiter: `(`, `{`, or `[`.
    Open(ast::Delimiter),
    /// An identifier.
    Ident(ast::LitSource),
    /// A label, like `'loop`.
    Label(ast::LitSource),
    /// A byte literal.
    Byte(ast::CopySource<u8>),
    /// A byte string literal, including escape sequences. Like `b"hello\nworld"`.
    ByteStr(ast::StrSource),
    /// A characer literal.
    Char(ast::CopySource<char>),
    /// A number literal, like `42` or `3.14` or `0xff`.
    Number(ast::NumberSource),
    /// A string literal, including escape sequences. Like `"hello\nworld"`.
    Str(ast::StrSource),
    /// A path with an associated item.
    IndexedPath(compile::ItemId),
    /// A constant block with an associated item.
    ConstBlock(compile::ItemId),
    /// An asynchronous block with an associated item.
    AsyncBlock(compile::ItemId),
    /// An indexed closure.
    Closure(compile::ItemId),
    /// An expanded macro.
    ExpandedMacro(parse::NonZeroId),
    /// The `abstract` keyword.
    Abstract,
    /// The `alignof` keyword.
    AlignOf,
    /// `&`.
    Amp,
    /// `&&`.
    AmpAmp,
    /// `&=`.
    AmpEq,
    /// `->`.
    Arrow,
    /// The `as` keyword.
    As,
    /// The `async` keyword.
    Async,
    /// `@`.
    At,
    /// The `await` keyword.
    Await,
    /// `!`.
    Bang,
    /// `!=`.
    BangEq,
    /// The `become` keyword.
    Become,
    /// The `break` keyword.
    Break,
    /// `^`.
    Caret,
    /// `^=`.
    CaretEq,
    /// `:`.
    Colon,
    /// `::`.
    ColonColon,
    /// `,`.
    Comma,
    /// The `const` keyword.
    Const,
    /// The `continue` keyword.
    Continue,
    /// The `crate` keyword.
    Crate,
    /// `-`.
    Dash,
    /// `-=`.
    DashEq,
    /// The `default` keyword.
    Default,
    /// `/`.
    Div,
    /// The `do` keyword.
    Do,
    /// `$`.
    Dollar,
    /// `.`.
    Dot,
    /// `..`.
    DotDot,
    /// `..=`.
    DotDotEq,
    /// The `else` keyword.
    Else,
    /// The `enum` keyword.
    Enum,
    /// `=`.
    Eq,
    /// `==`.
    EqEq,
    /// The `extern` keyword.
    Extern,
    /// The `false` keyword.
    False,
    /// The `final` keyword.
    Final,
    /// The `fn` keyword.
    Fn,
    /// The `for` keyword.
    For,
    /// `>`.
    Gt,
    /// `>=`.
    GtEq,
    /// `>>`.
    GtGt,
    /// `>>=`.
    GtGtEq,
    /// The `if` keyword.
    If,
    /// The `impl` keyword.
    Impl,
    /// The `in` keyword.
    In,
    /// The `is` keyword.
    Is,
    /// The `let` keyword.
    Let,
    /// The `loop` keyword.
    Loop,
    /// `<`.
    Lt,
    /// `<=`.
    LtEq,
    /// `<<`.
    LtLt,
    /// `<<=`.
    LtLtEq,
    /// The `macro` keyword.
    Macro,
    /// The `match` keyword.
    Match,
    /// The `mod` keyword.
    Mod,
    /// The `move` keyword.
    Move,
    /// The `mut` keyword.
    Mut,
    /// The `not` keyword.
    Not,
    /// The `offsetof` keyword.
    OffsetOf,
    /// The `override` keyword.
    Override,
    /// `%`.
    Perc,
    /// `%=`.
    PercEq,
    /// `|`.
    Pipe,
    /// |=`.
    PipeEq,
    /// `||`.
    PipePipe,
    /// `+`.
    Plus,
    /// `+=`.
    PlusEq,
    /// `#`.
    Pound,
    /// The `priv` keyword.
    Priv,
    /// The `proc` keyword.
    Proc,
    /// The `pub` keyword.
    Pub,
    /// The `pure` keyword.
    Pure,
    /// `?`.
    QuestionMark,
    /// The `ref` keyword.
    Ref,
    /// The `return` keyword.
    Return,
    /// `=>`.
    Rocket,
    /// The `select` keyword.
    Select,
    /// The `Self` keyword.
    SelfType,
    /// The `self` keyword.
    SelfValue,
    /// `;`.
    SemiColon,
    /// The `sizeof` keyword.
    SizeOf,
    /// `/=`.
    SlashEq,
    /// `*`.
    Star,
    /// `*=`.
    StarEq,
    /// The `static` keyword.
    Static,
    /// The `struct` keyword.
    Struct,
    /// The `super` keyword.
    Super,
    /// `~`.
    Tilde,
    /// The `true` keyword.
    True,
    /// The `typeof` keyword.
    TypeOf,
    /// `_`.
    Underscore,
    /// The `unsafe` keyword.
    Unsafe,
    /// The `use` keyword.
    Use,
    /// The `virtual` keyword.
    Virtual,
    /// The `while` keyword.
    While,
    /// The `yield` keyword.
    Yield,
    /// whitespace.
    Whitespace,
    /// a syntax root
    Root,
    /// a variable declaration
    Local,
    /// an item declaration
    Item,
    /// an enum declaration
    ItemEnum,
    /// a struct declaration
    ItemStruct,
    /// a constant item
    ItemConst,
    /// a function declaration
    ItemFn,
    /// an impl
    ItemImpl,
    /// a module declaration
    ItemMod,
    /// a file module declaration
    ItemFileMod,
    /// a use declaration
    ItemUse,
    /// a nested use path
    ItemUsePath,
    /// a nested use group
    ItemUseGroup,
    /// a variant
    Variant,
    /// a field declaration
    Field,
    /// an empty type body
    EmptyBody,
    /// a struct body
    StructBody,
    /// a tuple body
    TupleBody,
    /// a collection of function arguments
    FnArgs,
    /// a block
    Block,
    /// the body of a block
    BlockBody,
    /// an expression
    Expr,
    /// a chain of expressions
    ExprChain,
    /// a tuple expression
    ExprTuple,
    /// an array expression
    ExprArray,
    /// a unary expression
    ExprUnary,
    /// a binary expression
    ExprBinary,
    /// a group expression
    ExprGroup,
    /// an empty group expression
    ExprEmptyGroup,
    /// a try expression
    ExprTry,
    /// an indexing expression
    ExprIndex,
    /// a call expression
    ExprCall,
    /// a macro call expression
    ExprMacroCall,
    /// an anonymous object expression
    ExprObject,
    /// a match expression
    ExprMatch,
    /// a match arm
    ExprMatchArm,
    /// a select expression
    ExprSelect,
    /// a select arm
    ExprSelectArm,
    /// an `.await` expression
    ExprAwait,
    /// a field expression
    ExprField,
    /// the operator in an expression
    ExprOperator,
    /// an `if` expression
    ExprIf,
    /// the `else` part of an if-expression
    ExprElse,
    /// the `else if` part of an if-expression
    ExprElseIf,
    /// a `while` expression
    ExprWhile,
    /// a `loop` expression
    ExprLoop,
    /// a `break` expression
    ExprBreak,
    /// a `break` expression
    ExprContinue,
    /// a `return` expression
    ExprReturn,
    /// a `yield` expression
    ExprYield,
    /// a `for` expression
    ExprFor,
    /// a `<start>..<end>` expression
    ExprRange,
    /// a `<start>..=<end>` expression
    ExprRangeInclusive,
    /// a `..<end>` expression
    ExprRangeTo,
    /// a `..=<end>` expression
    ExprRangeToInclusive,
    /// a `<start>..` expression
    ExprRangeFrom,
    /// a `..` expression
    ExprRangeFull,
    /// an assign expression
    ExprAssign,
    /// a literal value
    Lit,
    /// a closure expression
    ExprClosure,
    /// a pattern
    Pat,
    /// an array pattern
    PatArray,
    /// a tuple pattern
    PatTuple,
    /// an object pattern
    PatObject,
    /// an ignore pattern
    PatIgnore,
    /// a path
    Path,
    /// the generics of a path
    PathGenerics,
    /// the `let` condition of a loop
    Condition,
    /// closure arguments
    ClosureArguments,
    /// an `#{` anonymous object key
    AnonymousObjectKey,
    /// an attribute
    Attribute,
    /// an inner attribute
    InnerAttribute,
    /// modifiers
    Modifiers,
    /// the `(super)` modifier
    ModifierSuper,
    /// the `(self)` modifier
    ModifierSelf,
    /// the `(crate)` modifier
    ModifierCrate,
    /// the `(in <path>)` modifier
    ModifierIn,
    /// a raw token stream
    TokenStream,
    /// a raw token stream
    TemplateString,
}

impl From<ast::Token> for Kind {
    fn from(token: ast::Token) -> Self {
        token.kind
    }
}

impl Kind {
    /// Try to convert an identifier into a keyword.
    pub(crate) fn from_keyword(ident: &str) -> Option<Self> {
        match ident {
            "abstract" => Some(Self::Abstract),
            "alignof" => Some(Self::AlignOf),
            "as" => Some(Self::As),
            "async" => Some(Self::Async),
            "await" => Some(Self::Await),
            "become" => Some(Self::Become),
            "break" => Some(Self::Break),
            "const" => Some(Self::Const),
            "continue" => Some(Self::Continue),
            "crate" => Some(Self::Crate),
            "default" => Some(Self::Default),
            "do" => Some(Self::Do),
            "else" => Some(Self::Else),
            "enum" => Some(Self::Enum),
            "extern" => Some(Self::Extern),
            "false" => Some(Self::False),
            "final" => Some(Self::Final),
            "fn" => Some(Self::Fn),
            "for" => Some(Self::For),
            "if" => Some(Self::If),
            "impl" => Some(Self::Impl),
            "in" => Some(Self::In),
            "is" => Some(Self::Is),
            "let" => Some(Self::Let),
            "loop" => Some(Self::Loop),
            "macro" => Some(Self::Macro),
            "match" => Some(Self::Match),
            "mod" => Some(Self::Mod),
            "move" => Some(Self::Move),
            "mut" => Some(Self::Mut),
            "not" => Some(Self::Not),
            "offsetof" => Some(Self::OffsetOf),
            "override" => Some(Self::Override),
            "priv" => Some(Self::Priv),
            "proc" => Some(Self::Proc),
            "pub" => Some(Self::Pub),
            "pure" => Some(Self::Pure),
            "ref" => Some(Self::Ref),
            "return" => Some(Self::Return),
            "select" => Some(Self::Select),
            "Self" => Some(Self::SelfType),
            "self" => Some(Self::SelfValue),
            "sizeof" => Some(Self::SizeOf),
            "static" => Some(Self::Static),
            "struct" => Some(Self::Struct),
            "super" => Some(Self::Super),
            "true" => Some(Self::True),
            "typeof" => Some(Self::TypeOf),
            "unsafe" => Some(Self::Unsafe),
            "use" => Some(Self::Use),
            "virtual" => Some(Self::Virtual),
            "while" => Some(Self::While),
            "yield" => Some(Self::Yield),
            _ => None,
        }
    }

    /// If applicable, convert this into a literal.
    pub(crate) fn as_literal_str(&self) -> Option<&'static str> {
        match self {
            Self::Close(d) => Some(d.close()),
            Self::Open(d) => Some(d.open()),
            Self::Abstract => Some("abstract"),
            Self::AlignOf => Some("alignof"),
            Self::As => Some("as"),
            Self::Async => Some("async"),
            Self::Await => Some("await"),
            Self::Become => Some("become"),
            Self::Break => Some("break"),
            Self::Const => Some("const"),
            Self::Continue => Some("continue"),
            Self::Crate => Some("crate"),
            Self::Default => Some("default"),
            Self::Do => Some("do"),
            Self::Else => Some("else"),
            Self::Enum => Some("enum"),
            Self::Extern => Some("extern"),
            Self::False => Some("false"),
            Self::Final => Some("final"),
            Self::Fn => Some("fn"),
            Self::For => Some("for"),
            Self::If => Some("if"),
            Self::Impl => Some("impl"),
            Self::In => Some("in"),
            Self::Is => Some("is"),
            Self::Let => Some("let"),
            Self::Loop => Some("loop"),
            Self::Macro => Some("macro"),
            Self::Match => Some("match"),
            Self::Mod => Some("mod"),
            Self::Move => Some("move"),
            Self::Mut => Some("mut"),
            Self::Not => Some("not"),
            Self::OffsetOf => Some("offsetof"),
            Self::Override => Some("override"),
            Self::Priv => Some("priv"),
            Self::Proc => Some("proc"),
            Self::Pub => Some("pub"),
            Self::Pure => Some("pure"),
            Self::Ref => Some("ref"),
            Self::Return => Some("return"),
            Self::Select => Some("select"),
            Self::SelfType => Some("Self"),
            Self::SelfValue => Some("self"),
            Self::SizeOf => Some("sizeof"),
            Self::Static => Some("static"),
            Self::Struct => Some("struct"),
            Self::Super => Some("super"),
            Self::True => Some("true"),
            Self::TypeOf => Some("typeof"),
            Self::Unsafe => Some("unsafe"),
            Self::Use => Some("use"),
            Self::Virtual => Some("virtual"),
            Self::While => Some("while"),
            Self::Yield => Some("yield"),
            Self::Amp => Some("&"),
            Self::AmpAmp => Some("&&"),
            Self::AmpEq => Some("&="),
            Self::Arrow => Some("->"),
            Self::At => Some("@"),
            Self::Bang => Some("!"),
            Self::BangEq => Some("!="),
            Self::Caret => Some("^"),
            Self::CaretEq => Some("^="),
            Self::Colon => Some(":"),
            Self::ColonColon => Some("::"),
            Self::Comma => Some(","),
            Self::Dash => Some("-"),
            Self::DashEq => Some("-="),
            Self::Div => Some("/"),
            Self::Dollar => Some("$"),
            Self::Dot => Some("."),
            Self::DotDot => Some(".."),
            Self::DotDotEq => Some("..="),
            Self::Eq => Some("="),
            Self::EqEq => Some("=="),
            Self::Gt => Some(">"),
            Self::GtEq => Some(">="),
            Self::GtGt => Some(">>"),
            Self::GtGtEq => Some(">>="),
            Self::Lt => Some("<"),
            Self::LtEq => Some("<="),
            Self::LtLt => Some("<<"),
            Self::LtLtEq => Some("<<="),
            Self::Perc => Some("%"),
            Self::PercEq => Some("%="),
            Self::Pipe => Some("|"),
            Self::PipeEq => Some("|="),
            Self::PipePipe => Some("||"),
            Self::Plus => Some("+"),
            Self::PlusEq => Some("+="),
            Self::Pound => Some("#"),
            Self::QuestionMark => Some("?"),
            Self::Rocket => Some("=>"),
            Self::SemiColon => Some(";"),
            Self::SlashEq => Some("/="),
            Self::Star => Some("*"),
            Self::StarEq => Some("*="),
            Self::Tilde => Some("~"),
            Self::Underscore => Some("_"),
            _ => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        parse::IntoExpectation::into_expectation(*self).fmt(f)
    }
}

impl macros::ToTokens for Kind {
    fn to_tokens(
        &self,
        context: &mut macros::MacroContext<'_, '_, '_>,
        stream: &mut macros::TokenStream,
    ) -> crate::alloc::Result<()> {
        stream.push(ast::Token {
            kind: *self,
            span: context.macro_span(),
        })
    }
}

impl parse::IntoExpectation for Kind {
    fn into_expectation(self) -> parse::Expectation {
        match self {
            Self::Eof => parse::Expectation::Description("eof"),
            Self::Comment | Self::MultilineComment(..) => parse::Expectation::Comment,
            Self::Error => parse::Expectation::Description("an error"),
            Self::Shebang { .. } => parse::Expectation::Description("a shebang"),
            Self::Ident(..) => parse::Expectation::Description("an identifier"),
            Self::Label(..) => parse::Expectation::Description("a label"),
            Self::Byte { .. } => parse::Expectation::Description("a byte literal"),
            Self::ByteStr { .. } => parse::Expectation::Description("a byte string literal"),
            Self::Char { .. } => parse::Expectation::Description("a character"),
            Self::Number { .. } => parse::Expectation::Description("a number"),
            Self::Str { .. } => parse::Expectation::Description("a string literal"),
            Self::Close(delimiter) => parse::Expectation::Delimiter(delimiter.close()),
            Self::Open(delimiter) => parse::Expectation::Delimiter(delimiter.open()),
            Self::IndexedPath(..) => parse::Expectation::Syntax("an indexed path"),
            Self::ConstBlock(..) => parse::Expectation::Syntax("a constant block"),
            Self::AsyncBlock(..) => parse::Expectation::Syntax("an asynchronous block"),
            Self::Closure(..) => parse::Expectation::Syntax("a closure"),
            Self::ExpandedMacro(..) => parse::Expectation::Syntax("an expanded macro"),
            Self::Abstract => parse::Expectation::Keyword("abstract"),
            Self::AlignOf => parse::Expectation::Keyword("alignof"),
            Self::As => parse::Expectation::Keyword("as"),
            Self::Async => parse::Expectation::Keyword("async"),
            Self::Await => parse::Expectation::Keyword("await"),
            Self::Become => parse::Expectation::Keyword("become"),
            Self::Break => parse::Expectation::Keyword("break"),
            Self::Const => parse::Expectation::Keyword("const"),
            Self::Continue => parse::Expectation::Keyword("continue"),
            Self::Crate => parse::Expectation::Keyword("crate"),
            Self::Default => parse::Expectation::Keyword("default"),
            Self::Do => parse::Expectation::Keyword("do"),
            Self::Else => parse::Expectation::Keyword("else"),
            Self::Enum => parse::Expectation::Keyword("enum"),
            Self::Extern => parse::Expectation::Keyword("extern"),
            Self::False => parse::Expectation::Keyword("false"),
            Self::Final => parse::Expectation::Keyword("final"),
            Self::Fn => parse::Expectation::Keyword("fn"),
            Self::For => parse::Expectation::Keyword("for"),
            Self::If => parse::Expectation::Keyword("if"),
            Self::Impl => parse::Expectation::Keyword("impl"),
            Self::In => parse::Expectation::Keyword("in"),
            Self::Is => parse::Expectation::Keyword("is"),
            Self::Let => parse::Expectation::Keyword("let"),
            Self::Loop => parse::Expectation::Keyword("loop"),
            Self::Macro => parse::Expectation::Keyword("macro"),
            Self::Match => parse::Expectation::Keyword("match"),
            Self::Mod => parse::Expectation::Keyword("mod"),
            Self::Move => parse::Expectation::Keyword("move"),
            Self::Mut => parse::Expectation::Keyword("mut"),
            Self::Not => parse::Expectation::Keyword("not"),
            Self::OffsetOf => parse::Expectation::Keyword("offsetof"),
            Self::Override => parse::Expectation::Keyword("override"),
            Self::Priv => parse::Expectation::Keyword("priv"),
            Self::Proc => parse::Expectation::Keyword("proc"),
            Self::Pub => parse::Expectation::Keyword("pub"),
            Self::Pure => parse::Expectation::Keyword("pure"),
            Self::Ref => parse::Expectation::Keyword("ref"),
            Self::Return => parse::Expectation::Keyword("return"),
            Self::Select => parse::Expectation::Keyword("select"),
            Self::SelfType => parse::Expectation::Keyword("Self"),
            Self::SelfValue => parse::Expectation::Keyword("self"),
            Self::SizeOf => parse::Expectation::Keyword("sizeof"),
            Self::Static => parse::Expectation::Keyword("static"),
            Self::Struct => parse::Expectation::Keyword("struct"),
            Self::Super => parse::Expectation::Keyword("super"),
            Self::True => parse::Expectation::Keyword("true"),
            Self::TypeOf => parse::Expectation::Keyword("typeof"),
            Self::Unsafe => parse::Expectation::Keyword("unsafe"),
            Self::Use => parse::Expectation::Keyword("use"),
            Self::Virtual => parse::Expectation::Keyword("virtual"),
            Self::While => parse::Expectation::Keyword("while"),
            Self::Yield => parse::Expectation::Keyword("yield"),
            Self::Amp => parse::Expectation::Punctuation("&"),
            Self::AmpAmp => parse::Expectation::Punctuation("&&"),
            Self::AmpEq => parse::Expectation::Punctuation("&="),
            Self::Arrow => parse::Expectation::Punctuation("->"),
            Self::At => parse::Expectation::Punctuation("@"),
            Self::Bang => parse::Expectation::Punctuation("!"),
            Self::BangEq => parse::Expectation::Punctuation("!="),
            Self::Caret => parse::Expectation::Punctuation("^"),
            Self::CaretEq => parse::Expectation::Punctuation("^="),
            Self::Colon => parse::Expectation::Punctuation(":"),
            Self::ColonColon => parse::Expectation::Punctuation("::"),
            Self::Comma => parse::Expectation::Punctuation(","),
            Self::Dash => parse::Expectation::Punctuation("-"),
            Self::DashEq => parse::Expectation::Punctuation("-="),
            Self::Div => parse::Expectation::Punctuation("/"),
            Self::Dollar => parse::Expectation::Punctuation("$"),
            Self::Dot => parse::Expectation::Punctuation("."),
            Self::DotDot => parse::Expectation::Punctuation(".."),
            Self::DotDotEq => parse::Expectation::Punctuation("..="),
            Self::Eq => parse::Expectation::Punctuation("="),
            Self::EqEq => parse::Expectation::Punctuation("=="),
            Self::Gt => parse::Expectation::Punctuation(">"),
            Self::GtEq => parse::Expectation::Punctuation(">="),
            Self::GtGt => parse::Expectation::Punctuation(">>"),
            Self::GtGtEq => parse::Expectation::Punctuation(">>="),
            Self::Lt => parse::Expectation::Punctuation("<"),
            Self::LtEq => parse::Expectation::Punctuation("<="),
            Self::LtLt => parse::Expectation::Punctuation("<<"),
            Self::LtLtEq => parse::Expectation::Punctuation("<<="),
            Self::Perc => parse::Expectation::Punctuation("%"),
            Self::PercEq => parse::Expectation::Punctuation("%="),
            Self::Pipe => parse::Expectation::Punctuation("|"),
            Self::PipeEq => parse::Expectation::Punctuation("|="),
            Self::PipePipe => parse::Expectation::Punctuation("||"),
            Self::Plus => parse::Expectation::Punctuation("+"),
            Self::PlusEq => parse::Expectation::Punctuation("+="),
            Self::Pound => parse::Expectation::Punctuation("#"),
            Self::QuestionMark => parse::Expectation::Punctuation("?"),
            Self::Rocket => parse::Expectation::Punctuation("=>"),
            Self::SemiColon => parse::Expectation::Punctuation(";"),
            Self::SlashEq => parse::Expectation::Punctuation("/="),
            Self::Star => parse::Expectation::Punctuation("*"),
            Self::StarEq => parse::Expectation::Punctuation("*="),
            Self::Tilde => parse::Expectation::Punctuation("~"),
            Self::Underscore => parse::Expectation::Punctuation("_"),
            Self::Whitespace => parse::Expectation::Syntax("whitespace."),
            Self::Root => parse::Expectation::Syntax("a syntax root"),
            Self::Local => parse::Expectation::Syntax("a variable declaration"),
            Self::Item => parse::Expectation::Syntax("an item declaration"),
            Self::ItemEnum => parse::Expectation::Syntax("an enum declaration"),
            Self::ItemStruct => parse::Expectation::Syntax("a struct declaration"),
            Self::ItemConst => parse::Expectation::Syntax("a constant item"),
            Self::ItemFn => parse::Expectation::Syntax("a function declaration"),
            Self::ItemImpl => parse::Expectation::Syntax("an impl"),
            Self::ItemMod => parse::Expectation::Syntax("a module declaration"),
            Self::ItemFileMod => parse::Expectation::Syntax("a file module declaration"),
            Self::ItemUse => parse::Expectation::Syntax("a use declaration"),
            Self::ItemUsePath => parse::Expectation::Syntax("a nested use path"),
            Self::ItemUseGroup => parse::Expectation::Syntax("a nested use group"),
            Self::Variant => parse::Expectation::Syntax("a variant"),
            Self::Field => parse::Expectation::Syntax("a field declaration"),
            Self::EmptyBody => parse::Expectation::Syntax("an empty type body"),
            Self::StructBody => parse::Expectation::Syntax("a struct body"),
            Self::TupleBody => parse::Expectation::Syntax("a tuple body"),
            Self::FnArgs => parse::Expectation::Syntax("a collection of function arguments"),
            Self::Block => parse::Expectation::Syntax("a block"),
            Self::BlockBody => parse::Expectation::Syntax("the body of a block"),
            Self::Expr => parse::Expectation::Syntax("an expression"),
            Self::ExprChain => parse::Expectation::Syntax("a chain of expressions"),
            Self::ExprTuple => parse::Expectation::Syntax("a tuple expression"),
            Self::ExprArray => parse::Expectation::Syntax("an array expression"),
            Self::ExprUnary => parse::Expectation::Syntax("a unary expression"),
            Self::ExprBinary => parse::Expectation::Syntax("a binary expression"),
            Self::ExprGroup => parse::Expectation::Syntax("a group expression"),
            Self::ExprEmptyGroup => parse::Expectation::Syntax("an empty group expression"),
            Self::ExprTry => parse::Expectation::Syntax("a try expression"),
            Self::ExprIndex => parse::Expectation::Syntax("an indexing expression"),
            Self::ExprCall => parse::Expectation::Syntax("a call expression"),
            Self::ExprMacroCall => parse::Expectation::Syntax("a macro call expression"),
            Self::ExprObject => parse::Expectation::Syntax("an anonymous object expression"),
            Self::ExprMatch => parse::Expectation::Syntax("a match expression"),
            Self::ExprMatchArm => parse::Expectation::Syntax("a match arm"),
            Self::ExprSelect => parse::Expectation::Syntax("a select expression"),
            Self::ExprSelectArm => parse::Expectation::Syntax("a select arm"),
            Self::ExprAwait => parse::Expectation::Syntax("an `.await` expression"),
            Self::ExprField => parse::Expectation::Syntax("a field expression"),
            Self::ExprOperator => parse::Expectation::Syntax("the operator in an expression"),
            Self::ExprIf => parse::Expectation::Syntax("an `if` expression"),
            Self::ExprElse => parse::Expectation::Syntax("the `else` part of an if-expression"),
            Self::ExprElseIf => {
                parse::Expectation::Syntax("the `else if` part of an if-expression")
            }
            Self::ExprWhile => parse::Expectation::Syntax("a `while` expression"),
            Self::ExprLoop => parse::Expectation::Syntax("a `loop` expression"),
            Self::ExprBreak => parse::Expectation::Syntax("a `break` expression"),
            Self::ExprContinue => parse::Expectation::Syntax("a `break` expression"),
            Self::ExprReturn => parse::Expectation::Syntax("a `return` expression"),
            Self::ExprYield => parse::Expectation::Syntax("a `yield` expression"),
            Self::ExprFor => parse::Expectation::Syntax("a `for` expression"),
            Self::ExprRange => parse::Expectation::Syntax("a `<start>..<end>` expression"),
            Self::ExprRangeInclusive => {
                parse::Expectation::Syntax("a `<start>..=<end>` expression")
            }
            Self::ExprRangeTo => parse::Expectation::Syntax("a `..<end>` expression"),
            Self::ExprRangeToInclusive => parse::Expectation::Syntax("a `..=<end>` expression"),
            Self::ExprRangeFrom => parse::Expectation::Syntax("a `<start>..` expression"),
            Self::ExprRangeFull => parse::Expectation::Syntax("a `..` expression"),
            Self::ExprAssign => parse::Expectation::Syntax("an assign expression"),
            Self::Lit => parse::Expectation::Syntax("a literal value"),
            Self::ExprClosure => parse::Expectation::Syntax("a closure expression"),
            Self::Pat => parse::Expectation::Syntax("a pattern"),
            Self::PatArray => parse::Expectation::Syntax("an array pattern"),
            Self::PatTuple => parse::Expectation::Syntax("a tuple pattern"),
            Self::PatObject => parse::Expectation::Syntax("an object pattern"),
            Self::PatIgnore => parse::Expectation::Syntax("an ignore pattern"),
            Self::Path => parse::Expectation::Syntax("a path"),
            Self::PathGenerics => parse::Expectation::Syntax("the generics of a path"),
            Self::Condition => parse::Expectation::Syntax("the `let` condition of a loop"),
            Self::ClosureArguments => parse::Expectation::Syntax("closure arguments"),
            Self::AnonymousObjectKey => parse::Expectation::Syntax("an `#{` anonymous object key"),
            Self::Attribute => parse::Expectation::Syntax("an attribute"),
            Self::InnerAttribute => parse::Expectation::Syntax("an inner attribute"),
            Self::Modifiers => parse::Expectation::Syntax("modifiers"),
            Self::ModifierSuper => parse::Expectation::Syntax("the `(super)` modifier"),
            Self::ModifierSelf => parse::Expectation::Syntax("the `(self)` modifier"),
            Self::ModifierCrate => parse::Expectation::Syntax("the `(crate)` modifier"),
            Self::ModifierIn => parse::Expectation::Syntax("the `(in <path>)` modifier"),
            Self::TokenStream => parse::Expectation::Syntax("a raw token stream"),
            Self::TemplateString => parse::Expectation::Syntax("a raw token stream"),
        }
    }
}
