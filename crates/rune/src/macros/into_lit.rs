use crate::alloc::{self, String};
use crate::ast;
use crate::macros::MacroContext;

/// Helper trait used for things that can be converted into tokens.
pub trait IntoLit {
    /// Convert the current thing into a token.
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit>;
}

impl<T> IntoLit for T
where
    ast::Number: From<T>,
{
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let id = cx.idx.q.storage.insert_number(self)?;
        let source = ast::NumberSource::Synthetic(id);
        Ok(ast::Lit::Number(ast::LitNumber { span, source }))
    }
}

impl IntoLit for char {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let source = ast::CopySource::Inline(self);
        Ok(ast::Lit::Char(ast::LitChar { span, source }))
    }
}

impl IntoLit for u8 {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let source = ast::CopySource::Inline(self);
        Ok(ast::Lit::Byte(ast::LitByte { span, source }))
    }
}

impl IntoLit for &str {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let id = cx.idx.q.storage.insert_str(self)?;
        let source = ast::StrSource::Synthetic(id);
        Ok(ast::Lit::Str(ast::LitStr { span, source }))
    }
}

impl IntoLit for &String {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        <&str>::into_lit(self, cx)
    }
}

impl IntoLit for String {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let id = cx.idx.q.storage.insert_string(self)?;
        let source = ast::StrSource::Synthetic(id);
        Ok(ast::Lit::Str(ast::LitStr { span, source }))
    }
}

impl IntoLit for &[u8] {
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        let span = cx.macro_span();
        let id = cx.idx.q.storage.insert_byte_string(self)?;
        let source = ast::StrSource::Synthetic(id);
        Ok(ast::Lit::ByteStr(ast::LitByteStr { span, source }))
    }
}

impl<const N: usize> IntoLit for [u8; N] {
    #[inline]
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        <&[u8]>::into_lit(&self[..], cx)
    }
}

impl<const N: usize> IntoLit for &[u8; N] {
    #[inline]
    fn into_lit(self, cx: &mut MacroContext<'_, '_, '_>) -> alloc::Result<ast::Lit> {
        <&[u8]>::into_lit(self, cx)
    }
}
