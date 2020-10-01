//! Macro compiler.

use crate::macros::{MacroContext, Storage, TokenStream};
use crate::query::{Query, QueryMod};
use crate::CompileResult;
use crate::{
    ast, CompileError, CompileErrorKind, Options, Parse, ParseError, Parser, Spanned as _,
};
use runestick::{Context, Hash, Item, Source};
use std::sync::Arc;

pub(crate) struct MacroCompiler<'a> {
    pub(crate) storage: Storage,
    pub(crate) item: &'a Item,
    pub(crate) mod_item: &'a QueryMod,
    pub(crate) macro_context: &'a mut MacroContext,
    pub(crate) options: &'a Options,
    pub(crate) context: &'a Context,
    pub(crate) source: Arc<Source>,
    pub(crate) query: &'a mut Query,
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
            &self.item,
            self.mod_item,
            None,
            &macro_call.path,
            &self.storage,
            &*self.source,
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

        self.macro_context.span = span;
        let result = handler(self.macro_context, input_stream);

        let output = match result {
            Ok(output) => output,
            Err(error) => {
                let error = match error.downcast::<ParseError>() {
                    Ok(error) => return Err(CompileError::from(error)),
                    Err(error) => error,
                };

                let error = match error.downcast::<runestick::SpannedError>() {
                    Ok(error) => {
                        return Err(CompileError::new(
                            error.span(),
                            CompileErrorKind::CallMacroError {
                                error: error.into_inner(),
                            },
                        ));
                    }
                    Err(error) => error,
                };

                return Err(CompileError::new(
                    span,
                    CompileErrorKind::CallMacroError { error },
                ));
            }
        };

        let token_stream = match output.downcast::<TokenStream>() {
            Ok(token_stream) => *token_stream,
            Err(..) => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::CallMacroError {
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
        parser.parse_eof()?;
        Ok(output)
    }
}
