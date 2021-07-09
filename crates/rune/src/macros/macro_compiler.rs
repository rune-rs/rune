//! Macro compiler.

use crate::macros::{MacroContext, Storage, TokenStream};
use crate::query::Query;
use crate::shared::Consts;
use crate::CompileResult;
use crate::{
    ast, CompileError, CompileErrorKind, IrError, Options, Parse, ParseError, Parser, Spanned as _,
};
use runestick::{CompileItem, Context, Hash, Source};
use std::sync::Arc;

pub(crate) struct MacroCompiler<'a> {
    pub(crate) item: Arc<CompileItem>,
    pub(crate) storage: Storage,
    pub(crate) options: &'a Options,
    pub(crate) context: &'a Context,
    pub(crate) source: Arc<Source>,
    pub(crate) query: Query,
    pub(crate) consts: Consts,
}

impl MacroCompiler<'_> {
    /// Compile the given macro into the given output type.
    pub(crate) fn eval_macro<T>(&mut self, macro_call: &ast::MacroCall) -> CompileResult<T>
    where
        T: Parse,
    {
        let span = macro_call.span();

        if !self.options.macros {
            return Err(CompileError::experimental(
                span,
                "macros must be enabled with `-O macros=true`",
            ));
        }

        // TODO: include information on the module the macro is being called
        // from.
        let named = self.query.convert_path(
            self.context,
            &self.storage,
            &*self.source,
            &macro_call.path,
        )?;

        let hash = Hash::type_hash(&named.item);

        let handler = match self.context.lookup_macro(hash) {
            Some(handler) => handler,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::MissingMacro { item: named.item },
                ));
            }
        };

        let input_stream = &macro_call.stream;

        let macro_context = MacroContext {
            macro_span: macro_call.span(),
            stream_span: macro_call.stream_span(),
            source: self.source.clone(),
            storage: self.storage.clone(),
            item: self.item.clone(),
            query: self.query.clone(),
            consts: self.consts.clone(),
        };

        let result = crate::macros::with_context(macro_context, || handler(input_stream));

        let output = match result {
            Ok(output) => output,
            Err(error) => {
                let error = match error.downcast::<ParseError>() {
                    Ok(error) => return Err(CompileError::from(error)),
                    Err(error) => error,
                };

                let error = match error.downcast::<IrError>() {
                    Ok(error) => return Err(CompileError::from(error)),
                    Err(error) => error,
                };

                let error = match error.downcast::<CompileError>() {
                    Ok(error) => return Err(error),
                    Err(error) => error,
                };

                let error = match error.downcast::<runestick::SpannedError>() {
                    Ok(error) => {
                        return Err(CompileError::new(
                            error.span(),
                            CompileErrorKind::CallMacroError {
                                item: named.item,
                                error: error.into_inner(),
                            },
                        ));
                    }
                    Err(error) => error,
                };

                return Err(CompileError::new(
                    span,
                    CompileErrorKind::CallMacroError {
                        item: named.item,
                        error,
                    },
                ));
            }
        };

        let token_stream = match output.downcast::<TokenStream>() {
            Ok(token_stream) => *token_stream,
            Err(..) => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::CallMacroError {
                        item: named.item,
                        error: runestick::Error::msg(format!(
                            "failed to downcast macro result, expected `{}`",
                            std::any::type_name::<TokenStream>()
                        )),
                    },
                ));
            }
        };

        let mut parser = Parser::from_token_stream(&token_stream);
        let output = parser.parse::<T>()?;
        parser.eof()?;

        Ok(output)
    }
}
