use crate::collections::HashMap;
use crate::compiling::{CompileError, CompileErrorKind, CompileVisitor};
use rune_ssa::Var;
use runestick::{SourceId, Span};
use std::rc::Rc;

/// A single scope.
#[derive(Debug, Default)]
pub struct Scope {
    /// Local variables in this scope and their associated value id.
    locals: HashMap<Box<str>, (Span, Var)>,
}

/// The stack of scopes being processed.
pub struct Stack {
    source_id: SourceId,
    visitor: Rc<dyn CompileVisitor>,
    stack: Vec<Scope>,
}

impl Stack {
    /// Construct a new stack with the given source id.
    pub(crate) fn new(source_id: SourceId, visitor: Rc<dyn CompileVisitor>) -> Self {
        Self {
            source_id,
            visitor,
            stack: vec![Scope::default()],
        }
    }

    /// Push a scope onto the stack.
    pub(crate) fn push(&mut self) {
        self.stack.push(Scope::default());
    }

    /// Pop a scope off the stack.
    pub(crate) fn pop(&mut self, span: Span) -> Result<(), CompileError> {
        if self.stack.pop().is_none() {
            return Err(CompileError::msg(
                span,
                "tried to pop empty stack of scopes",
            ));
        }

        Ok(())
    }

    /// Declare a variable in the scope.
    pub fn declare(&mut self, span: Span, name: &str, value: Var) -> Result<(), CompileError> {
        let scope = self.last_mut(span)?;

        if let Some((existing_span, _)) = scope.locals.insert(name.into(), (span, value)) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::VariableConflict {
                    name: name.into(),
                    existing_span,
                },
            ));
        }

        Ok(())
    }

    /// Get the variable corresponding to the given name.
    pub fn get(&self, span: Span, name: &str) -> Result<Var, CompileError> {
        for scope in self.stack.iter().rev() {
            if let Some((var_span, value_id)) = scope.locals.get(name).copied() {
                self.visitor
                    .visit_variable_use(self.source_id, var_span, span);
                return Ok(value_id);
            }
        }

        Err(CompileError::new(
            span,
            CompileErrorKind::MissingLocal {
                name: name.to_owned(),
            },
        ))
    }

    /// Access the last scope mutably.
    fn last_mut(&mut self, span: Span) -> Result<&mut Scope, CompileError> {
        if let Some(scope) = self.stack.last_mut() {
            Ok(scope)
        } else {
            Err(CompileError::msg(span, "empty stack of scopes"))
        }
    }
}
