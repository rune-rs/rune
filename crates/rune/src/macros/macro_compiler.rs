//! Macro compiler.

use crate::alloc::prelude::*;
use crate::ast;
use crate::ast::Spanned;
use crate::compile::{self, ErrorKind, ItemMeta};
use crate::indexing::Indexer;
use crate::macros::{MacroContext, ToTokens};
use crate::parse::{Parse, Parser};

use super::TokenStream;

pub(crate) struct MacroCompiler<'a, 'b, 'arena> {
    pub(crate) item_meta: ItemMeta,
    pub(crate) idx: &'a mut Indexer<'b, 'arena>,
}

impl MacroCompiler<'_, '_, '_> {
    /// Compile the given macro into the given output type.
    pub(crate) fn eval_macro<T>(&mut self, macro_call: &ast::MacroCall) -> compile::Result<T>
    where
        T: Parse,
    {
        let span = macro_call.span();

        if !self.idx.q.options.macros {
            return Err(compile::Error::msg(
                span,
                "macros must be enabled with `-O macros=true`",
            ));
        }

        let named = self.idx.q.convert_path(&macro_call.path)?;

        let hash = self.idx.q.pool.item_type_hash(named.item);

        let handler = match self.idx.q.context.lookup_macro(hash) {
            Some(handler) => handler,
            None => {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::MissingMacro {
                        item: self.idx.q.pool.item(named.item).try_to_owned()?,
                    },
                ));
            }
        };

        let input_stream = &macro_call.input;

        let token_stream = {
            let mut macro_context = MacroContext {
                macro_span: span,
                input_span: macro_call.input_span(),
                item_meta: self.item_meta,
                idx: self.idx,
            };

            handler(&mut macro_context, input_stream)?
        };

        let mut parser = Parser::from_token_stream(&token_stream, span);
        let output = parser.parse::<T>()?;
        parser.eof()?;

        Ok(output)
    }

    /// Compile the given macro into the given output type.
    pub(crate) fn eval_attribute_macro<T>(
        &mut self,
        attribute: &ast::Attribute,
        item: &ast::Item,
    ) -> compile::Result<Option<T>>
    where
        T: Parse,
    {
        let span = attribute.span();

        if !self.idx.q.options.macros {
            return Ok(None);
        }

        let named = self.idx.q.convert_path(&attribute.path)?;

        let hash = self.idx.q.pool.item_type_hash(named.item);

        let handler = match self.idx.q.context.lookup_attribute_macro(hash) {
            Some(handler) => handler,
            None => {
                return Ok(None);
            }
        };

        let input_stream = &attribute.input;

        let token_stream = {
            let mut macro_context = MacroContext {
                macro_span: attribute.span(),
                input_span: attribute.input_span(),
                item_meta: self.item_meta,
                idx: self.idx,
            };

            let mut item_stream = TokenStream::new();
            item.to_tokens(&mut macro_context, &mut item_stream)?;

            handler(&mut macro_context, input_stream, &item_stream)?
        };

        let mut parser = Parser::from_token_stream(&token_stream, span);

        parser.parse_all().map(Some)
    }
}
