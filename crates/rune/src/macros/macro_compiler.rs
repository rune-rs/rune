//! Macro compiler.

use crate::no_std::prelude::*;

use crate::ast;
use crate::ast::Spanned;
use crate::compile::{self, CompileErrorKind, ItemMeta, Options};
use crate::macros::{MacroContext, ToTokens};
use crate::parse::{Parse, Parser};
use crate::query::Query;
use crate::Context;

use super::TokenStream;

pub(crate) struct MacroCompiler<'a> {
    pub(crate) item_meta: ItemMeta,
    pub(crate) options: &'a Options,
    pub(crate) context: &'a Context,
    pub(crate) query: Query<'a>,
}

impl MacroCompiler<'_> {
    /// Compile the given macro into the given output type.
    pub(crate) fn eval_macro<T>(&mut self, macro_call: &ast::MacroCall) -> compile::Result<T>
    where
        T: Parse,
    {
        let span = macro_call.span();

        if !self.options.macros {
            return Err(compile::Error::msg(
                span,
                "macros must be enabled with `-O macros=true`",
            ));
        }

        // TODO: include information on the module the macro is being called
        // from.
        //
        // TODO: Figure out how to avoid performing ad-hoc lowering here.
        let arena = crate::hir::Arena::new();
        let ctx = crate::hir::lowering::Ctx::new(&arena, self.query.borrow());
        let path = crate::hir::lowering::path(&ctx, &macro_call.path)?;
        let named = self.query.convert_path(self.context, &path)?;

        let hash = self.query.pool.item_type_hash(named.item);

        let handler = match self.context.lookup_macro(hash) {
            Some(handler) => handler,
            None => {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::MissingMacro {
                        item: self.query.pool.item(named.item).to_owned(),
                    },
                ));
            }
        };

        let input_stream = &macro_call.stream;

        let token_stream = {
            let mut macro_context = MacroContext {
                macro_span: macro_call.span(),
                stream_span: macro_call.stream_span(),
                item_meta: self.item_meta,
                q: self.query.borrow(),
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
    ) -> compile::Result<T>
    where
        T: Parse,
    {
        let span = attribute.span();

        if !self.options.macros {
            return Err(compile::Error::msg(
                span,
                "macros must be enabled with `-O macros=true`",
            ));
        }

        // TODO: include information on the module the macro is being called
        // from.
        //
        // TODO: Figure out how to avoid performing ad-hoc lowering here.
        let arena = crate::hir::Arena::new();
        let ctx = crate::hir::lowering::Ctx::new(&arena, self.query.borrow());
        let path = crate::hir::lowering::path(&ctx, &attribute.path)?;
        let named = self.query.convert_path(self.context, &path)?;

        let hash = self.query.pool.item_type_hash(named.item);

        let handler = match self.context.lookup_attribute_macro(hash) {
            Some(handler) => handler,
            None => {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::MissingMacro {
                        item: self.query.pool.item(named.item).to_owned(),
                    },
                ));
            }
        };

        // TODO ATTRMCRO this should maybe remove the leading `=` or surrounding delimiter
        let input_stream = &attribute.input;

        let token_stream = {
            let mut macro_context = MacroContext {
                macro_span: attribute.span(),
                stream_span: attribute.stream_span(),
                item_meta: self.item_meta,
                q: self.query.borrow(),
            };

            let mut item_stream = TokenStream::new();
            item.to_tokens(&mut macro_context, &mut item_stream);

            handler(&mut macro_context, input_stream, &item_stream)?
        };

        let mut parser = Parser::from_token_stream(&token_stream, span);

        parser.parse_all()
    }
}
