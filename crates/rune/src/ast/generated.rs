use crate::ast;
use crate::macros;
use crate::parse;
use crate::shared;
use std::fmt;

/// This file has been generated from `assets\tokens.yaml`
/// DO NOT modify by hand!

/// The `abstract` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Abstract {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Abstract {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Abstract {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Abstract => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "abstract")),
        }
    }
}

impl parse::Peek for Abstract {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Abstract)
    }
}

impl macros::ToTokens for Abstract {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `alignof` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignOf {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for AlignOf {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for AlignOf {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AlignOf => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "alignof")),
        }
    }
}

impl parse::Peek for AlignOf {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AlignOf)
    }
}

impl macros::ToTokens for AlignOf {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `&`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amp {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Amp {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Amp {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Amp => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "&")),
        }
    }
}

impl parse::Peek for Amp {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Amp)
    }
}

impl macros::ToTokens for Amp {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `&&`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmpAmp {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for AmpAmp {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for AmpAmp {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AmpAmp => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "&&")),
        }
    }
}

impl parse::Peek for AmpAmp {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AmpAmp)
    }
}

impl macros::ToTokens for AmpAmp {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `&=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmpEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for AmpEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for AmpEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::AmpEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "&=")),
        }
    }
}

impl parse::Peek for AmpEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::AmpEq)
    }
}

impl macros::ToTokens for AmpEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `->`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Arrow {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Arrow {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Arrow => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "->")),
        }
    }
}

impl parse::Peek for Arrow {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Arrow)
    }
}

impl macros::ToTokens for Arrow {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `as` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct As {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for As {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for As {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::As => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "as")),
        }
    }
}

impl parse::Peek for As {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::As)
    }
}

impl macros::ToTokens for As {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `async` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Async {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Async {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Async {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Async => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "async")),
        }
    }
}

impl parse::Peek for Async {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Async)
    }
}

impl macros::ToTokens for Async {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `@`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for At {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for At {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::At => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "@")),
        }
    }
}

impl parse::Peek for At {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::At)
    }
}

impl macros::ToTokens for At {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `await` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Await {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Await {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Await {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Await => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "await")),
        }
    }
}

impl parse::Peek for Await {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Await)
    }
}

impl macros::ToTokens for Await {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bang {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Bang {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Bang {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Bang => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "!")),
        }
    }
}

impl parse::Peek for Bang {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Bang)
    }
}

impl macros::ToTokens for Bang {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `!=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BangEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for BangEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for BangEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::BangEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "!=")),
        }
    }
}

impl parse::Peek for BangEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::BangEq)
    }
}

impl macros::ToTokens for BangEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `become` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Become {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Become {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Become {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Become => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "become")),
        }
    }
}

impl parse::Peek for Become {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Become)
    }
}

impl macros::ToTokens for Become {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `break` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Break {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Break {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Break {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Break => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "break")),
        }
    }
}

impl parse::Peek for Break {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Break)
    }
}

impl macros::ToTokens for Break {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `^`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caret {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Caret {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Caret {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Caret => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "^")),
        }
    }
}

impl parse::Peek for Caret {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Caret)
    }
}

impl macros::ToTokens for Caret {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `^=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaretEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for CaretEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for CaretEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::CaretEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "^=")),
        }
    }
}

impl parse::Peek for CaretEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::CaretEq)
    }
}

impl macros::ToTokens for CaretEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colon {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Colon {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Colon {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Colon => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ":")),
        }
    }
}

impl parse::Peek for Colon {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Colon)
    }
}

impl macros::ToTokens for Colon {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `::`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColonColon {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for ColonColon {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for ColonColon {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::ColonColon => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "::")),
        }
    }
}

impl parse::Peek for ColonColon {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::ColonColon)
    }
}

impl macros::ToTokens for ColonColon {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `,`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comma {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Comma {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Comma {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Comma => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ",")),
        }
    }
}

impl parse::Peek for Comma {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Comma)
    }
}

impl macros::ToTokens for Comma {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `const` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Const {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Const {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Const {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Const => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "const")),
        }
    }
}

impl parse::Peek for Const {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Const)
    }
}

impl macros::ToTokens for Const {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `continue` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Continue {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Continue {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Continue {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Continue => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "continue")),
        }
    }
}

impl parse::Peek for Continue {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Continue)
    }
}

impl macros::ToTokens for Continue {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `crate` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crate {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Crate {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Crate {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Crate => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "crate")),
        }
    }
}

impl parse::Peek for Crate {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Crate)
    }
}

impl macros::ToTokens for Crate {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `-`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dash {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Dash {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Dash {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dash => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "-")),
        }
    }
}

impl parse::Peek for Dash {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dash)
    }
}

impl macros::ToTokens for Dash {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `-=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for DashEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for DashEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DashEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "-=")),
        }
    }
}

impl parse::Peek for DashEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DashEq)
    }
}

impl macros::ToTokens for DashEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `default` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Default {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Default {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Default {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Default => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "default")),
        }
    }
}

impl parse::Peek for Default {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Default)
    }
}

impl macros::ToTokens for Default {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Div {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Div {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Div {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Div => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "/")),
        }
    }
}

impl parse::Peek for Div {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Div)
    }
}

impl macros::ToTokens for Div {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `do` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Do {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Do {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Do {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Do => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "do")),
        }
    }
}

impl parse::Peek for Do {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Do)
    }
}

impl macros::ToTokens for Do {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `$`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dollar {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Dollar {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Dollar {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dollar => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "$")),
        }
    }
}

impl parse::Peek for Dollar {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dollar)
    }
}

impl macros::ToTokens for Dollar {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `.`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Dot {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Dot {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Dot => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ".")),
        }
    }
}

impl parse::Peek for Dot {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Dot)
    }
}

impl macros::ToTokens for Dot {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotDot {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for DotDot {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for DotDot {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DotDot => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "..")),
        }
    }
}

impl parse::Peek for DotDot {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DotDot)
    }
}

impl macros::ToTokens for DotDot {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `..=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotDotEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for DotDotEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for DotDotEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::DotDotEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "..=")),
        }
    }
}

impl parse::Peek for DotDotEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::DotDotEq)
    }
}

impl macros::ToTokens for DotDotEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `else` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Else {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Else {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Else {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Else => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "else")),
        }
    }
}

impl parse::Peek for Else {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Else)
    }
}

impl macros::ToTokens for Else {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `enum` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Enum {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Enum {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Enum {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Enum => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "enum")),
        }
    }
}

impl parse::Peek for Enum {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Enum)
    }
}

impl macros::ToTokens for Enum {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Eq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Eq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Eq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "=")),
        }
    }
}

impl parse::Peek for Eq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Eq)
    }
}

impl macros::ToTokens for Eq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `==`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for EqEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for EqEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::EqEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "==")),
        }
    }
}

impl parse::Peek for EqEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::EqEq)
    }
}

impl macros::ToTokens for EqEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `extern` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extern {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Extern {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Extern {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Extern => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "extern")),
        }
    }
}

impl parse::Peek for Extern {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Extern)
    }
}

impl macros::ToTokens for Extern {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `false` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct False {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for False {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for False {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::False => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "false")),
        }
    }
}

impl parse::Peek for False {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::False)
    }
}

impl macros::ToTokens for False {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `final` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Final {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Final {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Final {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Final => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "final")),
        }
    }
}

impl parse::Peek for Final {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Final)
    }
}

impl macros::ToTokens for Final {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `fn` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fn {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Fn {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Fn {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Fn => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "fn")),
        }
    }
}

impl parse::Peek for Fn {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Fn)
    }
}

impl macros::ToTokens for Fn {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `for` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct For {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for For {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for For {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::For => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "for")),
        }
    }
}

impl parse::Peek for For {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::For)
    }
}

impl macros::ToTokens for For {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gt {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Gt {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Gt {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Gt => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ">")),
        }
    }
}

impl parse::Peek for Gt {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Gt)
    }
}

impl macros::ToTokens for Gt {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `>=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for GtEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for GtEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ">=")),
        }
    }
}

impl parse::Peek for GtEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtEq)
    }
}

impl macros::ToTokens for GtEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `>>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtGt {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for GtGt {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for GtGt {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtGt => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ">>")),
        }
    }
}

impl parse::Peek for GtGt {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtGt)
    }
}

impl macros::ToTokens for GtGt {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `>>=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtGtEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for GtGtEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for GtGtEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::GtGtEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ">>=")),
        }
    }
}

impl parse::Peek for GtGtEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::GtGtEq)
    }
}

impl macros::ToTokens for GtGtEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `if` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct If {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for If {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for If {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::If => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "if")),
        }
    }
}

impl parse::Peek for If {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::If)
    }
}

impl macros::ToTokens for If {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `impl` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Impl {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Impl {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Impl {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Impl => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "impl")),
        }
    }
}

impl parse::Peek for Impl {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Impl)
    }
}

impl macros::ToTokens for Impl {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `in` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct In {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for In {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for In {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::In => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "in")),
        }
    }
}

impl parse::Peek for In {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::In)
    }
}

impl macros::ToTokens for In {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `is` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Is {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Is {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Is {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Is => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "is")),
        }
    }
}

impl parse::Peek for Is {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Is)
    }
}

impl macros::ToTokens for Is {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `let` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Let {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Let {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Let {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Let => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "let")),
        }
    }
}

impl parse::Peek for Let {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Let)
    }
}

impl macros::ToTokens for Let {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `loop` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loop {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Loop {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Loop {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Loop => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "loop")),
        }
    }
}

impl parse::Peek for Loop {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Loop)
    }
}

impl macros::ToTokens for Loop {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `<`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lt {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Lt {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Lt {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Lt => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "<")),
        }
    }
}

impl parse::Peek for Lt {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Lt)
    }
}

impl macros::ToTokens for Lt {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `<=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for LtEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for LtEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "<=")),
        }
    }
}

impl parse::Peek for LtEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtEq)
    }
}

impl macros::ToTokens for LtEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `<<`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtLt {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for LtLt {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for LtLt {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtLt => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "<<")),
        }
    }
}

impl parse::Peek for LtLt {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtLt)
    }
}

impl macros::ToTokens for LtLt {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `<<=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtLtEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for LtLtEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for LtLtEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::LtLtEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "<<=")),
        }
    }
}

impl parse::Peek for LtLtEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::LtLtEq)
    }
}

impl macros::ToTokens for LtLtEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `macro` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Macro {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Macro {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Macro {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Macro => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "macro")),
        }
    }
}

impl parse::Peek for Macro {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Macro)
    }
}

impl macros::ToTokens for Macro {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `match` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Match {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Match {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Match => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "match")),
        }
    }
}

impl parse::Peek for Match {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Match)
    }
}

impl macros::ToTokens for Match {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `mod` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mod {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Mod {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Mod {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Mod => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "mod")),
        }
    }
}

impl parse::Peek for Mod {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Mod)
    }
}

impl macros::ToTokens for Mod {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `move` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Move {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Move {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Move => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "move")),
        }
    }
}

impl parse::Peek for Move {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Move)
    }
}

impl macros::ToTokens for Move {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `not` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Not {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Not {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Not {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Not => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "not")),
        }
    }
}

impl parse::Peek for Not {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Not)
    }
}

impl macros::ToTokens for Not {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `offsetof` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetOf {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for OffsetOf {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for OffsetOf {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::OffsetOf => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "offsetof")),
        }
    }
}

impl parse::Peek for OffsetOf {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::OffsetOf)
    }
}

impl macros::ToTokens for OffsetOf {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `override` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Override {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Override {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Override {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Override => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "override")),
        }
    }
}

impl parse::Peek for Override {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Override)
    }
}

impl macros::ToTokens for Override {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `%`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Perc {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Perc {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Perc {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Perc => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "%")),
        }
    }
}

impl parse::Peek for Perc {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Perc)
    }
}

impl macros::ToTokens for Perc {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `%=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PercEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for PercEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for PercEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PercEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "%=")),
        }
    }
}

impl parse::Peek for PercEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PercEq)
    }
}

impl macros::ToTokens for PercEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `|`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pipe {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Pipe {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Pipe {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pipe => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "|")),
        }
    }
}

impl parse::Peek for Pipe {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pipe)
    }
}

impl macros::ToTokens for Pipe {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// |=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for PipeEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for PipeEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PipeEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "|=")),
        }
    }
}

impl parse::Peek for PipeEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PipeEq)
    }
}

impl macros::ToTokens for PipeEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `||`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipePipe {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for PipePipe {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for PipePipe {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PipePipe => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "||")),
        }
    }
}

impl parse::Peek for PipePipe {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PipePipe)
    }
}

impl macros::ToTokens for PipePipe {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `+`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plus {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Plus {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Plus {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Plus => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "+")),
        }
    }
}

impl parse::Peek for Plus {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Plus)
    }
}

impl macros::ToTokens for Plus {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `+=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlusEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for PlusEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for PlusEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::PlusEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "+=")),
        }
    }
}

impl parse::Peek for PlusEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::PlusEq)
    }
}

impl macros::ToTokens for PlusEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `#`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pound {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Pound {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Pound {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pound => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "#")),
        }
    }
}

impl parse::Peek for Pound {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pound)
    }
}

impl macros::ToTokens for Pound {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `priv` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priv {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Priv {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Priv {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Priv => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "priv")),
        }
    }
}

impl parse::Peek for Priv {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Priv)
    }
}

impl macros::ToTokens for Priv {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `proc` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proc {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Proc {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Proc {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Proc => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "proc")),
        }
    }
}

impl parse::Peek for Proc {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Proc)
    }
}

impl macros::ToTokens for Proc {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `pub` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pub {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Pub {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Pub {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pub => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "pub")),
        }
    }
}

impl parse::Peek for Pub {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pub)
    }
}

impl macros::ToTokens for Pub {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `pure` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pure {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Pure {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Pure {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Pure => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "pure")),
        }
    }
}

impl parse::Peek for Pure {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Pure)
    }
}

impl macros::ToTokens for Pure {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionMark {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for QuestionMark {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for QuestionMark {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::QuestionMark => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "?")),
        }
    }
}

impl parse::Peek for QuestionMark {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::QuestionMark)
    }
}

impl macros::ToTokens for QuestionMark {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `ref` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ref {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Ref {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Ref {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Ref => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "ref")),
        }
    }
}

impl parse::Peek for Ref {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Ref)
    }
}

impl macros::ToTokens for Ref {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `return` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Return {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Return {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Return {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Return => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "return")),
        }
    }
}

impl parse::Peek for Return {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Return)
    }
}

impl macros::ToTokens for Return {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `=>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rocket {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Rocket {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Rocket {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Rocket => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "=>")),
        }
    }
}

impl parse::Peek for Rocket {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Rocket)
    }
}

impl macros::ToTokens for Rocket {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `select` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Select {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Select {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Select {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Select => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "select")),
        }
    }
}

impl parse::Peek for Select {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Select)
    }
}

impl macros::ToTokens for Select {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `Self` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfType {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for SelfType {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for SelfType {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SelfType => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "Self")),
        }
    }
}

impl parse::Peek for SelfType {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SelfType)
    }
}

impl macros::ToTokens for SelfType {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `self` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfValue {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for SelfValue {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for SelfValue {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SelfValue => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "self")),
        }
    }
}

impl parse::Peek for SelfValue {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SelfValue)
    }
}

impl macros::ToTokens for SelfValue {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `;`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemiColon {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for SemiColon {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for SemiColon {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SemiColon => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, ";")),
        }
    }
}

impl parse::Peek for SemiColon {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SemiColon)
    }
}

impl macros::ToTokens for SemiColon {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `sizeof` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeOf {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for SizeOf {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for SizeOf {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SizeOf => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "sizeof")),
        }
    }
}

impl parse::Peek for SizeOf {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SizeOf)
    }
}

impl macros::ToTokens for SizeOf {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `/=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for SlashEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for SlashEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::SlashEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "/=")),
        }
    }
}

impl parse::Peek for SlashEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::SlashEq)
    }
}

impl macros::ToTokens for SlashEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Star {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Star {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Star {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Star => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "*")),
        }
    }
}

impl parse::Peek for Star {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Star)
    }
}

impl macros::ToTokens for Star {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `*=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarEq {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for StarEq {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for StarEq {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::StarEq => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "*=")),
        }
    }
}

impl parse::Peek for StarEq {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::StarEq)
    }
}

impl macros::ToTokens for StarEq {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `static` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Static {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Static {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Static {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Static => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "static")),
        }
    }
}

impl parse::Peek for Static {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Static)
    }
}

impl macros::ToTokens for Static {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `struct` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Struct {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Struct {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Struct {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Struct => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "struct")),
        }
    }
}

impl parse::Peek for Struct {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Struct)
    }
}

impl macros::ToTokens for Struct {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `super` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Super {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Super {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Super {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Super => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "super")),
        }
    }
}

impl parse::Peek for Super {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Super)
    }
}

impl macros::ToTokens for Super {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `~`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tilde {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Tilde {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Tilde {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Tilde => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "~")),
        }
    }
}

impl parse::Peek for Tilde {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Tilde)
    }
}

impl macros::ToTokens for Tilde {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `true` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct True {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for True {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for True {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::True => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "true")),
        }
    }
}

impl parse::Peek for True {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::True)
    }
}

impl macros::ToTokens for True {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `typeof` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeOf {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for TypeOf {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for TypeOf {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::TypeOf => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "typeof")),
        }
    }
}

impl parse::Peek for TypeOf {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::TypeOf)
    }
}

impl macros::ToTokens for TypeOf {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// `_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underscore {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Underscore {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Underscore {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Underscore => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "_")),
        }
    }
}

impl parse::Peek for Underscore {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Underscore)
    }
}

impl macros::ToTokens for Underscore {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `unsafe` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsafe {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Unsafe {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Unsafe {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Unsafe => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "unsafe")),
        }
    }
}

impl parse::Peek for Unsafe {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Unsafe)
    }
}

impl macros::ToTokens for Unsafe {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `use` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Use {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Use {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Use {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Use => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "use")),
        }
    }
}

impl parse::Peek for Use {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Use)
    }
}

impl macros::ToTokens for Use {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `virtual` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Virtual {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Virtual {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Virtual {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Virtual => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "virtual")),
        }
    }
}

impl parse::Peek for Virtual {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Virtual)
    }
}

impl macros::ToTokens for Virtual {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `while` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct While {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for While {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for While {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::While => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "while")),
        }
    }
}

impl parse::Peek for While {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::While)
    }
}

impl macros::ToTokens for While {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
    }
}

/// The `yield` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Yield {
    /// Associated token.
    pub token: ast::Token,
}

impl ast::Spanned for Yield {
    fn span(&self) -> ast::Span {
        self.token.span()
    }
}

impl parse::Parse for Yield {
    fn parse(p: &mut parse::Parser<'_>) -> Result<Self, parse::ParseError> {
        let token = p.next()?;

        match token.kind {
            ast::Kind::Yield => Ok(Self { token }),
            _ => Err(parse::ParseError::expected(&token, "yield")),
        }
    }
}

impl parse::Peek for Yield {
    fn peek(peeker: &mut parse::Peeker<'_>) -> bool {
        matches!(peeker.nth(0), ast::Kind::Yield)
    }
}

impl macros::ToTokens for Yield {
    fn to_tokens(&self, _: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(self.token);
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
    (abstract) => {
        $crate::ast::generated::Abstract
    };
    (alignof) => {
        $crate::ast::generated::AlignOf
    };
    (as) => {
        $crate::ast::generated::As
    };
    (async) => {
        $crate::ast::generated::Async
    };
    (await) => {
        $crate::ast::generated::Await
    };
    (become) => {
        $crate::ast::generated::Become
    };
    (break) => {
        $crate::ast::generated::Break
    };
    (const) => {
        $crate::ast::generated::Const
    };
    (continue) => {
        $crate::ast::generated::Continue
    };
    (crate) => {
        $crate::ast::generated::Crate
    };
    (default) => {
        $crate::ast::generated::Default
    };
    (do) => {
        $crate::ast::generated::Do
    };
    (else) => {
        $crate::ast::generated::Else
    };
    (enum) => {
        $crate::ast::generated::Enum
    };
    (extern) => {
        $crate::ast::generated::Extern
    };
    (false) => {
        $crate::ast::generated::False
    };
    (final) => {
        $crate::ast::generated::Final
    };
    (fn) => {
        $crate::ast::generated::Fn
    };
    (for) => {
        $crate::ast::generated::For
    };
    (if) => {
        $crate::ast::generated::If
    };
    (impl) => {
        $crate::ast::generated::Impl
    };
    (in) => {
        $crate::ast::generated::In
    };
    (is) => {
        $crate::ast::generated::Is
    };
    (let) => {
        $crate::ast::generated::Let
    };
    (loop) => {
        $crate::ast::generated::Loop
    };
    (macro) => {
        $crate::ast::generated::Macro
    };
    (match) => {
        $crate::ast::generated::Match
    };
    (mod) => {
        $crate::ast::generated::Mod
    };
    (move) => {
        $crate::ast::generated::Move
    };
    (not) => {
        $crate::ast::generated::Not
    };
    (offsetof) => {
        $crate::ast::generated::OffsetOf
    };
    (override) => {
        $crate::ast::generated::Override
    };
    (priv) => {
        $crate::ast::generated::Priv
    };
    (proc) => {
        $crate::ast::generated::Proc
    };
    (pub) => {
        $crate::ast::generated::Pub
    };
    (pure) => {
        $crate::ast::generated::Pure
    };
    (ref) => {
        $crate::ast::generated::Ref
    };
    (return) => {
        $crate::ast::generated::Return
    };
    (select) => {
        $crate::ast::generated::Select
    };
    (Self) => {
        $crate::ast::generated::SelfType
    };
    (self) => {
        $crate::ast::generated::SelfValue
    };
    (sizeof) => {
        $crate::ast::generated::SizeOf
    };
    (static) => {
        $crate::ast::generated::Static
    };
    (struct) => {
        $crate::ast::generated::Struct
    };
    (super) => {
        $crate::ast::generated::Super
    };
    (true) => {
        $crate::ast::generated::True
    };
    (typeof) => {
        $crate::ast::generated::TypeOf
    };
    (unsafe) => {
        $crate::ast::generated::Unsafe
    };
    (use) => {
        $crate::ast::generated::Use
    };
    (virtual) => {
        $crate::ast::generated::Virtual
    };
    (while) => {
        $crate::ast::generated::While
    };
    (yield) => {
        $crate::ast::generated::Yield
    };
    (&) => {
        $crate::ast::generated::Amp
    };
    (&&) => {
        $crate::ast::generated::AmpAmp
    };
    (&=) => {
        $crate::ast::generated::AmpEq
    };
    (->) => {
        $crate::ast::generated::Arrow
    };
    (@) => {
        $crate::ast::generated::At
    };
    (!) => {
        $crate::ast::generated::Bang
    };
    (!=) => {
        $crate::ast::generated::BangEq
    };
    (^) => {
        $crate::ast::generated::Caret
    };
    (^=) => {
        $crate::ast::generated::CaretEq
    };
    (:) => {
        $crate::ast::generated::Colon
    };
    (::) => {
        $crate::ast::generated::ColonColon
    };
    (,) => {
        $crate::ast::generated::Comma
    };
    (-) => {
        $crate::ast::generated::Dash
    };
    (-=) => {
        $crate::ast::generated::DashEq
    };
    (/) => {
        $crate::ast::generated::Div
    };
    ($) => {
        $crate::ast::generated::Dollar
    };
    (.) => {
        $crate::ast::generated::Dot
    };
    (..) => {
        $crate::ast::generated::DotDot
    };
    (..=) => {
        $crate::ast::generated::DotDotEq
    };
    (=) => {
        $crate::ast::generated::Eq
    };
    (==) => {
        $crate::ast::generated::EqEq
    };
    (>) => {
        $crate::ast::generated::Gt
    };
    (>=) => {
        $crate::ast::generated::GtEq
    };
    (>>) => {
        $crate::ast::generated::GtGt
    };
    (>>=) => {
        $crate::ast::generated::GtGtEq
    };
    (<) => {
        $crate::ast::generated::Lt
    };
    (<=) => {
        $crate::ast::generated::LtEq
    };
    (<<) => {
        $crate::ast::generated::LtLt
    };
    (<<=) => {
        $crate::ast::generated::LtLtEq
    };
    (%) => {
        $crate::ast::generated::Perc
    };
    (%=) => {
        $crate::ast::generated::PercEq
    };
    (|) => {
        $crate::ast::generated::Pipe
    };
    (|=) => {
        $crate::ast::generated::PipeEq
    };
    (||) => {
        $crate::ast::generated::PipePipe
    };
    (+) => {
        $crate::ast::generated::Plus
    };
    (+=) => {
        $crate::ast::generated::PlusEq
    };
    (#) => {
        $crate::ast::generated::Pound
    };
    (?) => {
        $crate::ast::generated::QuestionMark
    };
    (=>) => {
        $crate::ast::generated::Rocket
    };
    (;) => {
        $crate::ast::generated::SemiColon
    };
    (/=) => {
        $crate::ast::generated::SlashEq
    };
    (*) => {
        $crate::ast::generated::Star
    };
    (*=) => {
        $crate::ast::generated::StarEq
    };
    (~) => {
        $crate::ast::generated::Tilde
    };
    (_) => {
        $crate::ast::generated::Underscore
    };
}

/// Helper macro to reference a specific token kind, or short sequence of kinds.
#[macro_export]
macro_rules! K {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// En end-of-file marker.
    Eof,
    /// En error marker.
    Error,
    /// A close delimiter: `)`, `}`, or `]`.
    Close(ast::Delimiter),
    /// An open delimiter: `(`, `{`, or `[`.
    Open(ast::Delimiter),
    /// An identifier.
    Ident(ast::StringSource),
    /// A label, like `'loop`.
    Label(ast::StringSource),
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
}

impl From<ast::Token> for Kind {
    fn from(token: ast::Token) -> Self {
        token.kind
    }
}

impl Kind {
    /// Try to convert an identifier into a keyword.
    pub fn from_keyword(ident: &str) -> Option<Self> {
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

    /// Get the kind as a descriptive string.
    fn as_str(self) -> &'static str {
        match self {
            Self::Eof => "eof",
            Self::Error => "error",
            Self::Close(delimiter) => delimiter.close(),
            Self::Open(delimiter) => delimiter.open(),
            Self::Ident(..) => "ident",
            Self::Label(..) => "label",
            Self::Byte { .. } => "byte",
            Self::ByteStr { .. } => "byte string",
            Self::Char { .. } => "char",
            Self::Number { .. } => "number",
            Self::Str { .. } => "string",
            Self::Abstract => "abstract",
            Self::AlignOf => "alignof",
            Self::Amp => "&",
            Self::AmpAmp => "&&",
            Self::AmpEq => "&=",
            Self::Arrow => "->",
            Self::As => "as",
            Self::Async => "async",
            Self::At => "@",
            Self::Await => "await",
            Self::Bang => "!",
            Self::BangEq => "!=",
            Self::Become => "become",
            Self::Break => "break",
            Self::Caret => "^",
            Self::CaretEq => "^=",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Comma => ",",
            Self::Const => "const",
            Self::Continue => "continue",
            Self::Crate => "crate",
            Self::Dash => "-",
            Self::DashEq => "-=",
            Self::Default => "default",
            Self::Div => "/",
            Self::Do => "do",
            Self::Dollar => "$",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::DotDotEq => "..=",
            Self::Else => "else",
            Self::Enum => "enum",
            Self::Eq => "=",
            Self::EqEq => "==",
            Self::Extern => "extern",
            Self::False => "false",
            Self::Final => "final",
            Self::Fn => "fn",
            Self::For => "for",
            Self::Gt => ">",
            Self::GtEq => ">=",
            Self::GtGt => ">>",
            Self::GtGtEq => ">>=",
            Self::If => "if",
            Self::Impl => "impl",
            Self::In => "in",
            Self::Is => "is",
            Self::Let => "let",
            Self::Loop => "loop",
            Self::Lt => "<",
            Self::LtEq => "<=",
            Self::LtLt => "<<",
            Self::LtLtEq => "<<=",
            Self::Macro => "macro",
            Self::Match => "match",
            Self::Mod => "mod",
            Self::Move => "move",
            Self::Not => "not",
            Self::OffsetOf => "offsetof",
            Self::Override => "override",
            Self::Perc => "%",
            Self::PercEq => "%=",
            Self::Pipe => "|",
            Self::PipeEq => "|=",
            Self::PipePipe => "||",
            Self::Plus => "+",
            Self::PlusEq => "+=",
            Self::Pound => "#",
            Self::Priv => "priv",
            Self::Proc => "proc",
            Self::Pub => "pub",
            Self::Pure => "pure",
            Self::QuestionMark => "?",
            Self::Ref => "ref",
            Self::Return => "return",
            Self::Rocket => "=>",
            Self::Select => "select",
            Self::SelfType => "Self",
            Self::SelfValue => "self",
            Self::SemiColon => ";",
            Self::SizeOf => "sizeof",
            Self::SlashEq => "/=",
            Self::Star => "*",
            Self::StarEq => "*=",
            Self::Static => "static",
            Self::Struct => "struct",
            Self::Super => "super",
            Self::Tilde => "~",
            Self::True => "true",
            Self::TypeOf => "typeof",
            Self::Underscore => "_",
            Self::Unsafe => "unsafe",
            Self::Use => "use",
            Self::Virtual => "virtual",
            Self::While => "while",
            Self::Yield => "yield",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl macros::ToTokens for Kind {
    fn to_tokens(&self, context: &mut macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(ast::Token {
            kind: *self,
            span: context.macro_span(),
        });
    }
}

impl shared::Description for &Kind {
    fn description(self) -> &'static str {
        self.as_str()
    }
}
