use crate as rune;
use crate::alloc::clone;
use crate::ast;
use crate::compile;
use crate::macros;
use crate::parse;
use core::fmt;

/// This file has been generated from `assets\tokens.yaml`
/// DO NOT modify by hand!

/// The `abstract` keyword.
#[derive(Debug, clone::TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Abstract {
    /// Associated span.
    pub span: ast::Span,
}

impl ast::Spanned for Abstract {
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Abstract {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AlignOf {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Amp {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AmpAmp {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for AmpEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Arrow {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for As {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Async {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for At {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Await {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Bang {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for BangEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Become {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Break {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Caret {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for CaretEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Colon {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for ColonColon {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Comma {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Const {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Continue {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Crate {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dash {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DashEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Default {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Div {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Do {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dollar {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Dot {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DotDot {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for DotDotEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Else {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Enum {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Eq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for EqEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Extern {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for False {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Final {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Fn {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for For {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Gt {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtGt {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for GtGtEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for If {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Impl {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for In {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Is {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Let {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Loop {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Lt {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtLt {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for LtLtEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Macro {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Match {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Mod {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Move {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Mut {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Not {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for OffsetOf {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Override {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Perc {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PercEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pipe {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PipeEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PipePipe {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Plus {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for PlusEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pound {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Priv {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Proc {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pub {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Pure {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for QuestionMark {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Ref {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Return {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Rocket {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Select {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SelfType {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SelfValue {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SemiColon {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SizeOf {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for SlashEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Star {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for StarEq {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Static {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Struct {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Super {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Tilde {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for True {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for TypeOf {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Underscore {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Unsafe {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Use {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Virtual {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for While {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
    fn span(&self) -> ast::Span {
        self.span
    }
}

impl ast::OptionSpanned for Yield {
    fn option_span(&self) -> Option<ast::Span> {
        Some(self.span)
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
#[try_clone(copy)]
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
    /// Kind used for whitespace.
    Whitespace,
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
            Self::Error => parse::Expectation::Description("error"),
            Self::Shebang { .. } => parse::Expectation::Description("shebang"),
            Self::Ident(..) => parse::Expectation::Description("ident"),
            Self::Label(..) => parse::Expectation::Description("label"),
            Self::Byte { .. } => parse::Expectation::Description("byte literal"),
            Self::ByteStr { .. } => parse::Expectation::Description("byte string"),
            Self::Char { .. } => parse::Expectation::Description("char"),
            Self::Number { .. } => parse::Expectation::Description("number"),
            Self::Str { .. } => parse::Expectation::Description("string"),
            Self::Close(delimiter) => parse::Expectation::Delimiter(delimiter.close()),
            Self::Open(delimiter) => parse::Expectation::Delimiter(delimiter.open()),
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
            Self::Whitespace => parse::Expectation::Syntax,
        }
    }
}
