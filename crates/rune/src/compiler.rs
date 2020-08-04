use crate::ast;
use crate::collections::HashMap;
use crate::error::{CompileError, ConfigurationError};
use crate::source::Source;
use crate::traits::Resolve as _;
use crate::ParseAll;
use st::unit::{Assembly, Label, Span};

/// Compiler options.
pub struct Options {
    ///Enabled optimizations.
    pub optimizations: Optimizations,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            optimizations: Default::default(),
        }
    }
}

/// Optimizations enabled in the compiler.
pub struct Optimizations {
    /// Memoize the instance function in a loop.
    memoize_instance_fn: bool,
}

impl Optimizations {
    /// Parse the given option.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ConfigurationError> {
        let mut it = option.split('=');

        match it.next() {
            Some("memoize-instance-fn") => {
                self.memoize_instance_fn = it.next() != Some("false");
            }
            _ => {
                return Err(ConfigurationError::UnsupportedOptimizationOption {
                    option: option.to_owned(),
                });
            }
        }

        Ok(())
    }
}

impl Default for Optimizations {
    fn default() -> Self {
        Self {
            memoize_instance_fn: true,
        }
    }
}

/// Instance function to use for iteration.
const ITERATOR_NEXT: &str = "next";

type Result<T, E = CompileError> = std::result::Result<T, E>;

/// Flag to indicate if the expression should produce a value or not.
#[derive(Debug, Clone, Copy)]
struct NeedsValue(bool);

impl<'a> crate::ParseAll<'a, ast::File> {
    /// Compile the parse with default options.
    pub fn compile(self) -> Result<st::Unit> {
        self.compile_with_options(&Default::default())
    }

    /// Encode the given object into a collection of asm.
    pub fn compile_with_options(self, options: &Options) -> Result<st::Unit> {
        let ParseAll { source, item: file } = self;

        let mut unit = st::Unit::with_default_prelude();

        for import in file.imports {
            let name = resolve_path(import.path, source)?;
            unit.new_import(&name)?;
        }

        for f in file.functions {
            let name = f.name.resolve(source)?;
            let count = f.args.items.len();

            let mut assembly = unit.new_assembly();

            let mut encoder = Compiler {
                unit: &mut unit,
                asm: &mut assembly,
                scopes: vec![Scope::new()],
                source,
                loops: Vec::new(),
                references_at: Vec::new(),
                current_block: Span::empty(),
                optimizations: &options.optimizations,
            };

            encoder.encode_fn_decl(f)?;
            unit.new_function(&[name], count, assembly)?;
        }

        Ok(unit)
    }
}

struct Compiler<'a> {
    unit: &'a mut st::Unit,
    asm: &'a mut Assembly,
    scopes: Vec<Scope>,
    source: Source<'a>,
    /// The nesting of loop we are currently in.
    loops: Vec<Loop>,
    /// The current block that we are in.
    current_block: Span,
    /// Indicates that a reference was taken at the given spans.
    references_at: Vec<Span>,
    /// Enabled optimizations.
    optimizations: &'a Optimizations,
}

impl<'a> Compiler<'a> {
    fn encode_fn_decl(&mut self, fn_decl: ast::FnDecl) -> Result<()> {
        let scopes_count = self.scopes.len();
        let span = fn_decl.span();

        for arg in fn_decl.args.items.iter().rev() {
            let name = arg.resolve(self.source)?;
            self.new_var(name, arg.span(), Vec::new())?;
        }

        if fn_decl.body.exprs.is_empty() && fn_decl.body.trailing_expr.is_none() {
            self.asm.push(st::Inst::ReturnUnit, span);
            return Ok(());
        }

        for (expr, _) in &fn_decl.body.exprs {
            self.encode_expr(expr, NeedsValue(false))?;
        }

        if let Some(expr) = &fn_decl.body.trailing_expr {
            self.references_at.clear();
            self.encode_expr(expr, NeedsValue(true))?;

            if !self.references_at.is_empty() {
                return Err(CompileError::ReturnLocalReferences {
                    block: fn_decl.body.span(),
                    span: expr.span(),
                    references_at: self.references_at.clone(),
                });
            }

            let total_var_count = self.last_scope(span)?.total_var_count;
            self.clean_up_locals(total_var_count, span);
            self.asm.push(st::Inst::Return, span);
        } else {
            let total_var_count = self.last_scope(span)?.total_var_count;
            self.pop_locals(total_var_count, span);
            self.asm.push(st::Inst::ReturnUnit, span);
        }

        if self.scopes.len() != scopes_count {
            return Err(CompileError::internal(
                "number of scopes does not match at end of function",
                span,
            ));
        }

        Ok(())
    }

    /// Declare a new variable.
    fn new_var(&mut self, name: &str, span: Span, references_at: Vec<Span>) -> Result<()> {
        Ok(self
            .last_scope_mut(span)?
            .new_var(name, span, references_at)?)
    }

    /// Get the local with the given name.
    fn last_scope(&self, span: Span) -> Result<&Scope> {
        Ok(self
            .scopes
            .last()
            .ok_or_else(|| CompileError::internal("missing head of locals", span))?)
    }

    /// Get the local with the given name.
    fn get_var(&self, name: &str, span: Span) -> Result<&Var> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Ok(var);
            }
        }

        Err(CompileError::MissingLocal {
            name: name.to_owned(),
            span,
        })
    }

    /// Get the last locals scope.
    fn last_scope_mut(&mut self, span: Span) -> Result<&mut Scope> {
        Ok(self
            .scopes
            .last_mut()
            .ok_or_else(|| CompileError::internal("missing head of locals", span))?)
    }

    /// Get the local with the given name.
    fn get_var_mut(&mut self, name: &str, span: Span) -> Result<&mut Var> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                return Ok(var);
            }
        }

        Err(CompileError::MissingLocal {
            name: name.to_owned(),
            span,
        })
    }

    /// Pop locals by simply popping them.
    fn pop_locals(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(st::Inst::Pop, span);
            }
            count => {
                self.asm.push(st::Inst::PopN { count }, span);
            }
        }
    }

    /// Clean up local variables by preserving the value that is on top and
    /// popping the rest.
    ///
    /// The clean operation will preserve the value that is on top of the stack,
    /// and pop the values under it.
    fn clean_up_locals(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(st::Inst::Clean { count }, span);
            }
        }
    }

    /// Encode a block.
    ///
    /// Blocks are special in that they do not produce a value unless there is
    /// an item in them which does.
    fn encode_block(&mut self, block: &ast::Block, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", block);

        let span = block.span();
        self.current_block = span;

        let scopes_count = self.scopes.len();
        let new_scope = self.last_scope(span)?.new_scope();
        self.scopes.push(new_scope);

        for (expr, _) in &block.exprs {
            // NB: terminated expressions do not need to produce a value.
            self.encode_expr(expr, NeedsValue(false))?;
        }

        if let Some(expr) = &block.trailing_expr {
            self.references_at.clear();
            self.encode_expr(expr, needs_value)?;

            if needs_value.0 && !self.references_at.is_empty() {
                return Err(CompileError::ReturnLocalReferences {
                    block: self.current_block,
                    span: expr.span(),
                    references_at: self.references_at.clone(),
                });
            }
        }

        let scope = self.pop_scope(span)?;

        if needs_value.0 {
            if block.trailing_expr.is_none() {
                self.pop_locals(scope.local_var_count, span);
                self.asm.push(st::Inst::Unit, span);
            } else {
                self.clean_up_locals(scope.local_var_count, span);
            }
        } else {
            self.pop_locals(scope.local_var_count, span);
        }

        if self.scopes.len() != scopes_count {
            return Err(CompileError::internal(
                "parent scope mismatch at end of block",
                span,
            ));
        }

        Ok(())
    }

    /// Encode a return.
    fn encode_return(&mut self, return_: &ast::Return, needs_value: NeedsValue) -> Result<()> {
        let span = return_.span();
        // NB: we actually want total_var_count here since we need to clean up _every_
        // variable declared until we reached the current return.
        let total_var_count = self.last_scope(span)?.total_var_count;

        if needs_value.0 {
            return Err(CompileError::ReturnDoesNotProduceValue {
                block: self.current_block,
                span,
            });
        }

        if let Some(expr) = &return_.expr {
            self.encode_expr(&*expr, NeedsValue(true))?;
            self.clean_up_locals(total_var_count, span);
            self.asm.push(st::Inst::Return, span);
        } else {
            self.pop_locals(total_var_count, span);
            self.asm.push(st::Inst::ReturnUnit, span);
        }

        Ok(())
    }

    /// Encode an expression.
    fn encode_expr(&mut self, expr: &ast::Expr, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", expr);

        match expr {
            ast::Expr::ExprWhile(expr_while) => {
                self.encode_expr_while(expr_while, needs_value)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                self.encode_expr_for(expr_for, needs_value)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                self.encode_expr_loop(expr_loop, needs_value)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                self.encode_expr_let(expr_let, needs_value)?;
            }
            ast::Expr::IndexSet(index_set) => {
                self.encode_index_set(index_set, needs_value)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.encode_expr(&*expr.expr, needs_value)?;
            }
            ast::Expr::Ident(ident) => {
                self.encode_ident(ident, needs_value)?;
            }
            ast::Expr::Path(path) => {
                self.encode_type(path, needs_value)?;
            }
            ast::Expr::CallFn(call_fn) => {
                self.encode_call_fn(call_fn, needs_value)?;
            }
            ast::Expr::CallInstanceFn(call_instance_fn) => {
                self.encode_call_instance_fn(call_instance_fn, needs_value)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.encode_expr_unary(expr_unary, needs_value)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.encode_expr_binary(expr_binary, needs_value)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.encode_expr_if(expr_if, needs_value)?;
            }
            ast::Expr::UnitLiteral(unit) => {
                self.encode_unit_literal(unit)?;
            }
            ast::Expr::BoolLiteral(b) => {
                self.encode_bool_literal(b, needs_value)?;
            }
            ast::Expr::NumberLiteral(number) => {
                self.encode_number_literal(number, needs_value)?;
            }
            ast::Expr::ArrayLiteral(array_literal) => {
                self.encode_array_literal(array_literal, needs_value)?;
            }
            ast::Expr::ObjectLiteral(object_literal) => {
                self.encode_object_literal(object_literal, needs_value)?;
            }
            ast::Expr::CharLiteral(string) => {
                self.encode_char_literal(string, needs_value)?;
            }
            ast::Expr::StringLiteral(string) => {
                self.encode_string_literal(string, needs_value)?;
            }
            ast::Expr::IndexGet(index_get) => {
                self.encode_index_get(index_get, needs_value)?;
            }
            ast::Expr::Break(b) => {
                self.encode_break(b, needs_value)?;
            }
            ast::Expr::Block(b) => {
                self.encode_block(b, needs_value)?;
            }
            ast::Expr::Return(return_) => {
                self.encode_return(return_, needs_value)?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                self.encode_expr_match(expr_match, needs_value)?;
            }
        }

        Ok(())
    }

    fn encode_array_literal(
        &mut self,
        array_literal: &ast::ArrayLiteral,
        needs_value: NeedsValue,
    ) -> Result<()> {
        if !needs_value.0 && array_literal.is_all_literal() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let count = array_literal.items.len();

        for expr in array_literal.items.iter().rev() {
            self.encode_expr(expr, NeedsValue(true))?;
        }

        self.asm
            .push(st::Inst::Array { count }, array_literal.span());
        Ok(())
    }

    fn encode_object_literal(
        &mut self,
        object: &ast::ObjectLiteral,
        needs_value: NeedsValue,
    ) -> Result<()> {
        if !needs_value.0 {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let mut keys = Vec::new();
        let mut keys_dup = HashMap::new();

        for (key, _, _) in &object.items {
            let span = key.span();
            let key = key.resolve(self.source)?;
            keys.push(key.to_string());

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: object.span(),
                });
            }
        }

        for (_, _, value) in object.items.iter().rev() {
            self.encode_expr(value, NeedsValue(true))?;
        }

        let slot = self.unit.new_static_object_keys(&keys)?;

        self.asm.push(st::Inst::Object { slot }, object.span());
        Ok(())
    }

    /// Encode a char literal, like `'a'`.
    fn encode_char_literal(&mut self, c: &ast::CharLiteral, needs_value: NeedsValue) -> Result<()> {
        // NB: Elide the entire literal if it's not needed.
        if !needs_value.0 {
            return Ok(());
        }

        let resolved_char = c.resolve(self.source)?;
        self.asm
            .push(st::Inst::Char { c: resolved_char }, c.token.span);
        Ok(())
    }

    /// Encode a string literal, like `"foo bar"`.
    fn encode_string_literal(
        &mut self,
        string: &ast::StringLiteral,
        needs_value: NeedsValue,
    ) -> Result<()> {
        // NB: Elide the entire literal if it's not needed.
        if !needs_value.0 {
            return Ok(());
        }

        let span = string.span();
        let string = string.resolve(self.source)?;
        let slot = self.unit.new_static_string(&*string)?;
        self.asm.push(st::Inst::String { slot }, span);
        Ok(())
    }

    fn encode_unit_literal(&mut self, literal: &ast::UnitLiteral) -> Result<()> {
        self.asm.push(st::Inst::Unit, literal.span());
        Ok(())
    }

    fn encode_bool_literal(&mut self, b: &ast::BoolLiteral, needs_value: NeedsValue) -> Result<()> {
        // If the value is not needed, no need to encode it.
        if !needs_value.0 {
            return Ok(());
        }

        self.asm.push(st::Inst::Bool { value: b.value }, b.span());
        Ok(())
    }

    fn encode_number_literal(
        &mut self,
        number: &ast::NumberLiteral,
        needs_value: NeedsValue,
    ) -> Result<()> {
        if !needs_value.0 {
            // NB: don't encode unecessary literal.
            return Ok(());
        }

        let span = number.span();
        let number = number.resolve(self.source)?;

        match number {
            ast::Number::Float(number) => {
                self.asm.push(st::Inst::Float { number }, span);
            }
            ast::Number::Integer(number) => {
                self.asm.push(st::Inst::Integer { number }, span);
            }
        }

        Ok(())
    }

    fn encode_expr_while(
        &mut self,
        expr_while: &ast::ExprWhile,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", expr_while);

        let span = expr_while.span();

        let start_label = self.asm.new_label("while_test");
        let end_label = self.asm.new_label("while_end");

        let loop_count = self.loops.len();

        self.loops.push(Loop {
            end_label,
            total_var_count: self.last_scope(span)?.total_var_count,
        });

        self.asm.label(start_label)?;
        self.encode_expr(&*expr_while.condition, NeedsValue(true))?;
        self.asm.jump_if_not(end_label, span);
        self.encode_block(&*expr_while.body, NeedsValue(false))?;

        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // NB: If a value is needed from a while loop, encode it as a unit.
        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        if self.loops.pop().is_none() {
            return Err(CompileError::internal("while: missing parent loop", span));
        }

        if loop_count != self.loops.len() {
            return Err(CompileError::internal(
                "while: loop count mismatch on return",
                span,
            ));
        }

        Ok(())
    }

    fn encode_expr_for(&mut self, expr_for: &ast::ExprFor, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", expr_for);

        let span = expr_for.span();

        let start_label = self.asm.new_label("for_start");
        let end_label = self.asm.new_label("for_end");

        let loop_count = self.loops.len();
        let scopes_count = self.scopes.len();

        let new_scope = self.last_scope(span)?.new_scope();
        self.scopes.push(new_scope);

        self.references_at.clear();
        self.encode_expr(&*expr_for.iter, NeedsValue(true))?;
        let references_at = self.references_at.clone();

        // Declare storage for the hidden iterator variable.
        let iterator_offset = self.last_scope_mut(span)?.decl_anon(expr_for.iter.span());

        // Declare named loop variable.
        let binding_offset = {
            self.asm.push(st::Inst::Unit, expr_for.iter.span());
            let name = expr_for.var.resolve(self.source)?;
            self.last_scope_mut(span)?
                .decl_var(name, expr_for.var.span(), references_at)
        };

        // Declare storage for memoized `next` instance fn.
        let next_offset = if self.optimizations.memoize_instance_fn {
            let offset = self.last_scope_mut(span)?.decl_anon(expr_for.iter.span());
            let hash = st::Hash::of(ITERATOR_NEXT);

            // Declare the named loop variable and put it in the scope.
            self.asm.push(
                st::Inst::Copy {
                    offset: iterator_offset,
                },
                expr_for.iter.span(),
            );

            self.asm
                .push(st::Inst::LoadInstanceFn { hash }, expr_for.iter.span());
            Some(offset)
        } else {
            None
        };

        self.loops.push(Loop {
            end_label,
            total_var_count: self.last_scope(span)?.total_var_count,
        });

        self.asm.label(start_label)?;

        // Use the memoized loop variable.
        if let Some(next_offset) = next_offset {
            self.asm.push(
                st::Inst::Copy {
                    offset: iterator_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push(
                st::Inst::Copy {
                    offset: next_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push(st::Inst::CallFn { args: 0 }, span);

            self.asm.push(
                st::Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        } else {
            // call the `next` function to get the next level of iteration, bind the
            // result to the loop variable in the loop.
            self.asm.push(
                st::Inst::Copy {
                    offset: iterator_offset,
                },
                expr_for.iter.span(),
            );

            let hash = st::Hash::of(ITERATOR_NEXT);
            self.asm
                .push(st::Inst::CallInstance { hash, args: 0 }, span);
            self.asm.push(
                st::Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        }

        // test loop condition.
        {
            self.asm.push(
                st::Inst::Copy {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
            self.asm.push(st::Inst::IsUnit, expr_for.span());
            self.asm.jump_if(end_label, expr_for.span());
        }

        self.encode_block(&*expr_for.body, NeedsValue(false))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        let total_var_count = match self.scopes.pop() {
            Some(scope) => scope.local_var_count,
            None => {
                return Err(CompileError::internal("scopes are imbalanced", span));
            }
        };

        self.pop_locals(total_var_count, span);

        // NB: If a value is needed from a for loop, encode it as a unit.
        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        if self.loops.pop().is_none() {
            return Err(CompileError::internal("for: missing parent loop", span));
        }

        if loop_count != self.loops.len() {
            return Err(CompileError::internal(
                "for: loop count mismatch on return",
                span,
            ));
        }

        if scopes_count != self.scopes.len() {
            return Err(CompileError::internal(
                "scope count mismatch on return",
                span,
            ));
        }

        Ok(())
    }

    fn encode_expr_loop(
        &mut self,
        expr_loop: &ast::ExprLoop,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", expr_loop);

        let span = expr_loop.span();

        let start_label = self.asm.new_label("loop_start");
        let end_label = self.asm.new_label("loop_end");

        let loop_count = self.loops.len();

        self.loops.push(Loop {
            end_label,
            total_var_count: self.last_scope(span)?.total_var_count,
        });

        self.asm.label(start_label)?;
        self.encode_block(&*expr_loop.body, NeedsValue(false))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // NB: If a value is needed from a while loop, encode it as a unit.
        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        if self.loops.pop().is_none() {
            return Err(CompileError::internal("loop: missing parent loop", span));
        }

        if loop_count != self.loops.len() {
            return Err(CompileError::internal(
                "loop: loop count mismatch on return",
                span,
            ));
        }

        Ok(())
    }

    fn encode_expr_let(&mut self, expr_let: &ast::ExprLet, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", expr_let);

        let span = expr_let.span();

        let name = expr_let.name.resolve(self.source)?;

        self.references_at.clear();
        self.encode_expr(&*expr_let.expr, NeedsValue(true))?;
        let references_at = self.references_at.clone();

        self.last_scope_mut(span)?
            .decl_var(name, expr_let.name.span(), references_at);

        // If a value is needed for a let expression, it is evaluated as a unit.
        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    /// Push reference on the stack for replacement.
    fn encode_assign_target(&mut self, expr: &ast::Expr, first_level: bool) -> Result<()> {
        match expr {
            ast::Expr::Ident(ident) => {
                self.encode_ident(ident, NeedsValue(true))?;
                return Ok(());
            }
            ast::Expr::ExprUnary(unary) => match unary.op {
                ast::UnaryOp::Deref { token } => {
                    self.encode_assign_target(&*unary.expr, false)?;

                    if !first_level {
                        self.asm.push(st::Inst::Deref, token.span);
                    }

                    return Ok(());
                }
                _ => (),
            },
            _ => (),
        }

        Err(CompileError::UnsupportedAssignExpr { span: expr.span() })
    }

    fn encode_assign(
        &mut self,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?} = {:?}", lhs, rhs);

        let span = lhs.span().join(rhs.span());

        match lhs {
            ast::Expr::Ident(ident) => {
                let name = ident.resolve(self.source)?;

                self.references_at.clear();
                self.encode_expr(rhs, NeedsValue(true))?;

                let references_at = self.references_at.clone().into_iter();

                let var = self.get_var_mut(name, ident.span())?;
                var.references_at.extend(references_at);
                let offset = var.offset;

                self.asm.push(st::Inst::Replace { offset }, span);
            }
            lhs => {
                self.encode_expr(rhs, NeedsValue(true))?;
                self.encode_assign_target(lhs, true)?;
                self.asm.push(st::Inst::ReplaceDeref, span);
            }
        }

        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    fn encode_index_get(
        &mut self,
        index_get: &ast::IndexGet,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", index_get);
        let span = index_get.span();

        self.encode_expr(&*index_get.index, NeedsValue(true))?;
        self.encode_expr(&*index_get.target, NeedsValue(true))?;
        self.asm.push(st::Inst::IndexGet, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs_value.0 {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a `break` expression.
    fn encode_break(&mut self, b: &ast::Break, needs_value: NeedsValue) -> Result<()> {
        let span = b.span();

        if needs_value.0 {
            return Err(CompileError::BreakDoesNotProduceValue { span });
        }

        let last_loop = match self.loops.last().copied() {
            Some(last_loop) => last_loop,
            None => {
                return Err(CompileError::BreakOutsideOfLoop { span });
            }
        };

        let locals = self.last_scope(span)?;

        let vars = locals
            .total_var_count
            .checked_sub(last_loop.total_var_count)
            .ok_or_else(|| CompileError::internal("var count should be larger", span))?;

        self.pop_locals(vars, span);
        self.asm.jump(last_loop.end_label, span);
        // NB: loops are expected to produce a value at the end of their expression.
        Ok(())
    }

    fn encode_index_set(
        &mut self,
        index_set: &ast::IndexSet,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", index_set);
        let span = index_set.span();

        self.encode_expr(&*index_set.value, NeedsValue(true))?;
        self.encode_expr(&*index_set.index, NeedsValue(true))?;
        self.encode_expr(&*index_set.target, NeedsValue(true))?;
        self.asm.push(st::Inst::IndexSet, span);

        // Encode a unit in case a value is needed.
        if needs_value.0 {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    /// Encode a local copy.
    fn encode_ident(&mut self, ident: &ast::Ident, needs_value: NeedsValue) -> Result<()> {
        log::trace!("encode local: {:?}", ident);

        // NB: avoid the encode completely if it is not needed.
        if !needs_value.0 {
            return Ok(());
        }

        let target = ident.resolve(self.source)?;
        let var = match self.get_var(target, ident.token.span) {
            Ok(var) => var,
            Err(..) => {
                // Something imported is automatically a type.
                if let Some(path) = self.unit.lookup_import_by_name(target) {
                    let hash = st::Hash::of_type(path);
                    self.asm.push(st::Inst::Type { hash }, ident.span());
                    return Ok(());
                }

                return Err(CompileError::MissingLocal {
                    name: target.to_owned(),
                    span: ident.token.span,
                });
            }
        };

        let references_at = var.references_at.clone().into_iter();
        let offset = var.offset;

        self.references_at.extend(references_at);
        self.asm.push(st::Inst::Copy { offset }, ident.span());
        Ok(())
    }

    /// Decode a path into a call destination based on its hashes.
    fn decode_call_dest(&self, path: &ast::Path) -> Result<st::Hash> {
        let local = path.first.resolve(self.source)?;

        let imported = match self.unit.lookup_import_by_name(local).cloned() {
            Some(path) => path,
            None => st::Item::of(&[local]),
        };

        let mut rest = Vec::new();

        for (_, part) in &path.rest {
            rest.push(part.resolve(self.source)?);
        }

        let it = imported
            .into_iter()
            .map(String::as_str)
            .chain(rest.into_iter());

        Ok(st::Hash::function(it))
    }

    /// Encode the given type.
    fn encode_type(&mut self, path: &ast::Path, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", path);

        // NB: do nothing if we don't need a value.
        if !needs_value.0 {
            return Ok(());
        }

        let mut parts = Vec::new();
        parts.push(path.first.resolve(self.source)?);

        for (_, part) in &path.rest {
            parts.push(part.resolve(self.source)?);
        }

        let hash = st::Hash::of_type(&parts);
        self.asm.push(st::Inst::Type { hash }, path.span());
        Ok(())
    }

    fn encode_call_fn(&mut self, call_fn: &ast::CallFn, needs_value: NeedsValue) -> Result<()> {
        log::trace!("{:?}", call_fn);

        let span = call_fn.span();
        let args = call_fn.args.items.len();

        for expr in call_fn.args.items.iter().rev() {
            self.encode_expr(expr, NeedsValue(true))?;
        }

        let hash = self.decode_call_dest(&call_fn.name)?;
        self.asm.push(st::Inst::Call { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs_value.0 {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn encode_call_instance_fn(
        &mut self,
        call_instance_fn: &ast::CallInstanceFn,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", call_instance_fn);

        let span = call_instance_fn.span();
        let args = call_instance_fn.args.items.len();

        for expr in call_instance_fn.args.items.iter().rev() {
            self.encode_expr(expr, NeedsValue(true))?;
        }

        self.encode_expr(&*call_instance_fn.instance, NeedsValue(true))?;

        let name = call_instance_fn.name.resolve(self.source)?;
        let hash = st::Hash::of(name);
        self.asm.push(st::Inst::CallInstance { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs_value.0 {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn encode_expr_unary(
        &mut self,
        expr_unary: &ast::ExprUnary,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", expr_unary);
        let span = expr_unary.span();

        // NB: special unary expressions.
        match expr_unary.op {
            ast::UnaryOp::Ref { .. } => {
                self.encode_ref(&*expr_unary.expr, expr_unary.span())?;
                return Ok(());
            }
            _ => (),
        }

        self.encode_expr(&*expr_unary.expr, NeedsValue(true))?;

        match expr_unary.op {
            ast::UnaryOp::Not { .. } => {
                self.asm.push(st::Inst::Not, span);
            }
            ast::UnaryOp::Deref { .. } => {
                self.asm.push(st::Inst::Deref, span);
            }
            op => {
                return Err(CompileError::UnsupportedUnaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs_value.0 {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a ref `&<expr>` value.
    fn encode_ref(&mut self, expr: &ast::Expr, span: Span) -> Result<()> {
        match expr {
            ast::Expr::Ident(ident) => {
                let target = ident.resolve(self.source)?;

                let offset = self.get_var(target, ident.token.span)?.offset;

                self.references_at.push(span);
                self.asm.push(st::Inst::Ptr { offset }, span);
            }
            _ => {
                return Err(CompileError::UnsupportedRef { span });
            }
        }

        Ok(())
    }

    fn encode_expr_binary(
        &mut self,
        expr_binary: &ast::ExprBinary,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", expr_binary);

        // Special expressions which operates on the stack in special ways.
        match expr_binary.op {
            ast::BinOp::Assign { .. } => {
                self.encode_assign(&*expr_binary.lhs, &*expr_binary.rhs, needs_value)?;
                return Ok(());
            }
            _ => (),
        }

        let span = expr_binary.span();

        self.encode_expr(&*expr_binary.lhs, NeedsValue(true))?;
        self.encode_expr(&*expr_binary.rhs, NeedsValue(true))?;

        match expr_binary.op {
            ast::BinOp::Add { .. } => {
                self.asm.push(st::Inst::Add, span);
            }
            ast::BinOp::Sub { .. } => {
                self.asm.push(st::Inst::Sub, span);
            }
            ast::BinOp::Div { .. } => {
                self.asm.push(st::Inst::Div, span);
            }
            ast::BinOp::Mul { .. } => {
                self.asm.push(st::Inst::Mul, span);
            }
            ast::BinOp::Eq { .. } => {
                self.asm.push(st::Inst::Eq, span);
            }
            ast::BinOp::Neq { .. } => {
                self.asm.push(st::Inst::Neq, span);
            }
            ast::BinOp::Lt { .. } => {
                self.asm.push(st::Inst::Lt, span);
            }
            ast::BinOp::Gt { .. } => {
                self.asm.push(st::Inst::Gt, span);
            }
            ast::BinOp::Lte { .. } => {
                self.asm.push(st::Inst::Lte, span);
            }
            ast::BinOp::Gte { .. } => {
                self.asm.push(st::Inst::Gte, span);
            }
            ast::BinOp::Is { .. } => {
                self.asm.push(st::Inst::Is, span);
            }
            op => {
                return Err(CompileError::UnsupportedBinaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs_value.0 {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn encode_expr_if(&mut self, expr_if: &ast::ExprIf, needs_value: NeedsValue) -> Result<()> {
        let span = expr_if.span();

        log::trace!("{:?}", expr_if);

        let then_label = self.asm.new_label("if_then");
        let end_label = self.asm.new_label("if_end");

        let mut branch_labels = Vec::new();

        self.encode_expr(&*expr_if.condition, NeedsValue(true))?;
        self.asm.jump_if(then_label, span);

        for branch in &expr_if.expr_else_ifs {
            let label = self.asm.new_label("if_branch");
            branch_labels.push(label);

            self.encode_expr(&*branch.condition, needs_value)?;
            self.asm.jump_if(label, branch.span());
        }

        // use fallback as fall through.
        if let Some(fallback) = &expr_if.expr_else {
            self.encode_block(&*fallback.block, needs_value)?;
        } else {
            // NB: if we must produce a value and there is no fallback branch,
            // encode the result of the statement as a unit.
            if needs_value.0 {
                self.asm.push(st::Inst::Unit, span);
            }
        }

        self.asm.jump(end_label, span);

        self.asm.label(then_label)?;
        self.encode_block(&*expr_if.block, needs_value)?;

        if !expr_if.expr_else_ifs.is_empty() {
            self.asm.jump(end_label, span);
        }

        let mut it = expr_if
            .expr_else_ifs
            .iter()
            .zip(branch_labels.iter().copied())
            .peekable();

        if let Some((branch, label)) = it.next() {
            let span = branch.span();
            self.asm.label(label)?;
            self.encode_block(&*branch.block, needs_value)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;
        Ok(())
    }

    fn encode_expr_match(
        &mut self,
        expr_match: &ast::ExprMatch,
        needs_value: NeedsValue,
    ) -> Result<()> {
        log::trace!("{:?}", expr_match);
        let span = expr_match.span();

        let new_scope = self.last_scope(span)?.new_scope();
        self.scopes.push(new_scope);

        self.encode_expr(&*expr_match.expr, NeedsValue(true))?;
        // Offset of the expression.
        let offset = self.last_scope_mut(span)?.decl_anon(expr_match.expr.span());

        let end_label = self.asm.new_label("match_end");
        let mut branches = Vec::new();

        for (branch, _) in &expr_match.branches {
            let span = branch.span();

            let branch_label = self.asm.new_label("match_branch");
            let match_false = self.asm.new_label("match_false");

            let mut scope = self.last_scope(span)?.new_scope();

            let load = Box::new(move |asm: &mut Assembly| {
                asm.push(st::Inst::Copy { offset }, span);
            });

            self.encode_pat(&mut scope, &branch.pat, match_false, &*load)?;

            self.asm.jump(branch_label, span);
            self.asm.label(match_false)?;

            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if !expr_match.has_default {
            if needs_value.0 {
                self.asm.push(st::Inst::Unit, span);
            }

            self.asm.jump(end_label, span);
        }

        let mut it = expr_match.branches.iter().zip(&branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            self.asm.label(*label)?;

            self.scopes.push(scope.clone());
            self.encode_expr(&*branch.body, needs_value)?;
            let scope = self.pop_scope(span)?;

            if needs_value.0 {
                self.clean_up_locals(scope.local_var_count, span);
            } else {
                self.pop_locals(scope.local_var_count, span);
            }

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;

        // pop the implicit scope where we store the anonymous match variable.
        let scope = self.pop_scope(span)?;

        println!("local var count: {}", scope.local_var_count);

        if needs_value.0 {
            self.clean_up_locals(scope.local_var_count, span);
        } else {
            self.pop_locals(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Encode an array pattern match.
    fn encode_array_pat(
        &mut self,
        scope: &mut Scope,
        array: &ast::ArrayPat,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = array.span();

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed an array.
        {
            load(&mut self.asm);

            if array.open_pattern.is_some() {
                self.asm.push(
                    st::Inst::MatchArray {
                        len: array.items.len(),
                        exact: false,
                    },
                    span,
                );
            } else {
                self.asm.push(
                    st::Inst::MatchArray {
                        len: array.items.len(),
                        exact: true,
                    },
                    span,
                );
            }
        }

        let length_true = self.asm.new_label("pat_array_len_true");

        self.asm.jump_if(length_true, span);
        self.pop_locals(scope.local_var_count, span);
        self.asm.jump(false_label, span);
        self.asm.label(length_true)?;

        for (index, (pat, _)) in array.items.iter().enumerate() {
            let span = pat.span();

            let load = Box::new(move |asm: &mut Assembly| {
                load(asm);
                asm.push(st::Inst::ArrayIndexGet { index }, span);
            });

            self.encode_pat(scope, &*pat, false_label, &*load)?;
        }

        Ok(())
    }

    /// Encode an object pattern match.
    fn encode_object_pat(
        &mut self,
        scope: &mut Scope,
        object: &ast::ObjectPat,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let mut string_slots = Vec::new();
        let span = object.span();

        let mut keys_dup = HashMap::new();
        let mut keys = Vec::new();

        for (string, _, _, _) in &object.items {
            let span = string.span();

            let string = string.resolve(self.source)?;
            string_slots.push(self.unit.new_static_string(&*string)?);
            keys.push(string.to_string());

            if let Some(existing) = keys_dup.insert(string, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: object.span(),
                });
            }
        }

        let keys = self.unit.new_static_object_keys(&keys[..])?;

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed an array.
        {
            load(&mut self.asm);

            if object.open_pattern.is_some() {
                self.asm.push(
                    st::Inst::MatchObject {
                        slot: keys,
                        exact: false,
                    },
                    span,
                );
            } else {
                self.asm.push(
                    st::Inst::MatchObject {
                        slot: keys,
                        exact: true,
                    },
                    span,
                );
            }
        }

        let length_true = self.asm.new_label("pat_object_len_true");

        self.asm.jump_if(length_true, span);
        self.pop_locals(scope.local_var_count, span);
        self.asm.jump(false_label, span);
        self.asm.label(length_true)?;

        for ((_, _, pat, _), slot) in object.items.iter().zip(string_slots) {
            let span = pat.span();

            let load = Box::new(move |asm: &mut Assembly| {
                load(asm);
                asm.push(st::Inst::ObjectSlotIndexGet { slot }, span);
            });

            // load the given array index and declare it as a local variable.
            self.encode_pat(scope, &*pat, false_label, &*load)?;
        }

        Ok(())
    }

    /// Encode a pattern.
    ///
    /// Patterns will clean up their own locals and execute a jump to
    /// `false_label` in case the pattern does not match.
    ///
    /// This label is typically a fallthrough to the next pattern.
    fn encode_pat(
        &mut self,
        scope: &mut Scope,
        pat: &ast::Pat,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        log::trace!("{:?}", pat);
        let span = pat.span();

        let true_label = self.asm.new_label("pat_true");

        match pat {
            ast::Pat::BindingPat(binding) => {
                load(&mut self.asm);
                let name = binding.resolve(self.source)?;
                scope.new_var(name, span, Vec::new())?;
                return Ok(());
            }
            ast::Pat::IgnorePat(..) => {
                return Ok(());
            }
            ast::Pat::UnitPat(unit) => {
                let span = unit.span();

                load(&mut self.asm);
                self.asm.push(st::Inst::IsUnit, span);
                self.asm.jump_if(true_label, span);
            }
            ast::Pat::CharPat(char_literal) => {
                let span = char_literal.span();

                let character = char_literal.resolve(self.source)?;

                load(&mut self.asm);
                self.asm.push(st::Inst::EqCharacter { character }, span);
                self.asm.jump_if(true_label, span);
            }
            ast::Pat::NumberPat(number_literal) => {
                let span = number_literal.span();

                let number = number_literal.resolve(self.source)?;

                let integer = match number {
                    ast::Number::Integer(integer) => integer,
                    ast::Number::Float(..) => {
                        return Err(CompileError::MatchFloatInPattern { span });
                    }
                };

                load(&mut self.asm);
                self.asm.push(st::Inst::EqInteger { integer }, span);

                self.asm.jump_if(true_label, span);
            }
            ast::Pat::StringPat(string) => {
                let span = string.span();

                let string = string.resolve(self.source)?;
                let slot = self.unit.new_static_string(&*string)?;

                load(&mut self.asm);
                self.asm.push(st::Inst::EqStaticString { slot }, span);

                self.asm.jump_if(true_label, span);
            }
            ast::Pat::ArrayPat(array) => {
                self.encode_array_pat(scope, array, false_label, load)?;
                return Ok(());
            }
            ast::Pat::ObjectPat(object) => {
                self.encode_object_pat(scope, object, false_label, load)?;
                return Ok(());
            }
        }

        // default method of cleaning up locals.
        let count = scope.local_var_count;
        self.pop_locals(count, span);
        self.asm.jump(false_label, span);
        self.asm.label(true_label)?;

        Ok(())
    }

    /// Pop the last scope.
    fn pop_scope(&mut self, span: Span) -> Result<Scope> {
        match self.scopes.pop() {
            Some(scope) => Ok(scope),
            None => {
                return Err(CompileError::internal("missing parent scope", span));
            }
        }
    }
}

/// Decode the specified path.
fn resolve_path<'a>(path: ast::Path, source: Source<'a>) -> Result<Vec<&'a str>> {
    let mut output = Vec::new();

    output.push(path.first.resolve(source)?);

    for (_, ident) in path.rest {
        output.push(ident.resolve(source)?);
    }

    Ok(output)
}

/// A locally declared variable.
#[derive(Debug, Clone)]
struct Var {
    /// Slot offset from the current stack frame.
    offset: usize,
    /// Name of the variable.
    name: String,
    /// Token assocaited with the variable.
    span: Span,
    /// Var references used by local expression.
    references_at: Vec<Span>,
}

/// A locally declared variable.
#[derive(Debug, Clone)]
struct AnonVar {
    /// Slot offset from the current stack frame.
    offset: usize,
    /// Span associated with the anonymous variable.
    span: Span,
}

#[derive(Debug, Clone)]
struct Scope {
    /// Named variables.
    locals: HashMap<String, Var>,
    /// Anonymous variables.
    anon: Vec<AnonVar>,
    /// The number of variables.
    total_var_count: usize,
    /// The number of variables local to this scope.
    local_var_count: usize,
}

impl Scope {
    /// Construct a new locals handlers.
    pub fn new() -> Scope {
        Self {
            locals: HashMap::new(),
            anon: Vec::new(),
            total_var_count: 0,
            local_var_count: 0,
        }
    }

    /// Construct a new scope.
    pub fn new_scope(&self) -> Self {
        Self {
            locals: HashMap::new(),
            anon: Vec::new(),
            total_var_count: self.total_var_count,
            local_var_count: 0,
        }
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub fn new_var(&mut self, name: &str, span: Span, references_at: Vec<Span>) -> Result<()> {
        let local = Var {
            offset: self.total_var_count,
            name: name.to_owned(),
            span,
            references_at,
        };

        self.total_var_count += 1;
        self.local_var_count += 1;

        if let Some(old) = self.locals.insert(name.to_owned(), local) {
            return Err(CompileError::VariableConflict {
                name: name.to_owned(),
                span,
                existing_span: old.span,
            });
        }

        Ok(())
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub fn decl_var(&mut self, name: &str, span: Span, references_at: Vec<Span>) -> usize {
        let offset = self.total_var_count;

        self.locals.insert(
            name.to_owned(),
            Var {
                offset,
                name: name.to_owned(),
                span,
                references_at,
            },
        );

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Insert a new local, and return the old one if there's a conflict.
    fn decl_anon(&mut self, span: Span) -> usize {
        let offset = self.total_var_count;

        self.anon.push(AnonVar { offset, span });

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Access the local with the given name.
    pub fn get(&self, name: &str) -> Option<&Var> {
        if let Some(local) = self.locals.get(name) {
            return Some(local);
        }

        None
    }

    /// Access the local with the given name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Var> {
        if let Some(local) = self.locals.get_mut(name) {
            return Some(local);
        }

        None
    }
}

/// Loops we are inside.
#[derive(Clone, Copy)]
struct Loop {
    /// The end label of the loop.
    end_label: Label,
    /// The number of variables observed at the start of the loop.
    total_var_count: usize,
}
