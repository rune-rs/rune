use crate::ast;
use crate::collections::HashMap;
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use crate::ParseAll;
use st::unit::{Assembly, Label, Span};

mod loops;
mod options;
mod scopes;
mod warning;

use self::loops::{Loop, Loops};
pub use self::options::Options;
use self::scopes::{Scope, ScopeGuard, Scopes};
pub use self::warning::{Warning, Warnings};

/// Instance function to use for iteration.
const ITERATOR_NEXT: &str = "next";

type Result<T, E = CompileError> = std::result::Result<T, E>;

/// Flag to indicate if the expression should produce a value or not.
#[derive(Debug, Clone, Copy)]
struct NeedsValue(bool);

impl std::ops::Deref for NeedsValue {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> crate::ParseAll<'a, ast::DeclFile> {
    /// Compile the parse with default options.
    pub fn compile(self) -> Result<(st::CompilationUnit, Warnings)> {
        self.compile_with_options(&Default::default())
    }

    /// Encode the given object into a collection of asm.
    pub fn compile_with_options(
        self,
        options: &Options,
    ) -> Result<(st::CompilationUnit, Warnings)> {
        let ParseAll { source, item: file } = self;

        let mut warnings = Warnings::new();

        let mut unit = st::CompilationUnit::with_default_prelude();

        for import in file.imports {
            let name = import.path.resolve(source)?;
            unit.new_import(&name)?;
        }

        for f in file.functions {
            let span = f.span();
            let name = f.name.resolve(source)?;
            let count = f.args.items.len();

            let mut assembly = unit.new_assembly();

            let mut encoder = Compiler {
                unit: &mut unit,
                asm: &mut assembly,
                scopes: Scopes::new(),
                contexts: vec![span],
                source,
                loops: Loops::new(),
                current_block: Span::empty(),
                options,
                warnings: &mut warnings,
            };

            encoder.compile_decl_fn(f)?;
            unit.new_function(&[name], count, assembly)?;
        }

        Ok((unit, warnings))
    }
}

struct Compiler<'a> {
    unit: &'a mut st::CompilationUnit,
    asm: &'a mut Assembly,
    scopes: Scopes,
    /// Context for which to emit warnings.
    contexts: Vec<Span>,
    source: Source<'a>,
    /// The nesting of loop we are currently in.
    loops: Loops,
    /// The current block that we are in.
    current_block: Span,
    /// Enabled optimizations.
    options: &'a Options,
    /// Compilation warnings.
    warnings: &'a mut Warnings,
}

impl<'a> Compiler<'a> {
    fn compile_decl_fn(&mut self, fn_decl: ast::DeclFn) -> Result<()> {
        let span = fn_decl.span();
        log::trace!("FnDecl => {:?}", self.source.source(span)?);

        for (arg, _) in fn_decl.args.items.iter().rev() {
            let name = arg.resolve(self.source)?;
            self.scopes.last_mut(span)?.new_var(name, arg.span())?;
        }

        if fn_decl.body.exprs.is_empty() && fn_decl.body.trailing_expr.is_none() {
            self.asm.push(st::Inst::ReturnUnit, span);
            return Ok(());
        }

        for (expr, _) in &fn_decl.body.exprs {
            self.compile_expr(expr, NeedsValue(false))?;
        }

        if let Some(expr) = &fn_decl.body.trailing_expr {
            self.compile_expr(expr, NeedsValue(true))?;

            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_clean(total_var_count, span);
            self.asm.push(st::Inst::Return, span);
        } else {
            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_pop(total_var_count, span);
            self.asm.push(st::Inst::ReturnUnit, span);
        }

        self.scopes.pop_last(span)?;
        Ok(())
    }

    /// Pop locals by simply popping them.
    fn locals_pop(&mut self, total_var_count: usize, span: Span) {
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
    fn locals_clean(&mut self, total_var_count: usize, span: Span) {
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
    fn compile_expr_block(
        &mut self,
        block: &ast::ExprBlock,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = block.span();
        log::trace!("ExprBlock => {:?}", self.source.source(span)?);

        self.contexts.push(span);

        let span = block.span();
        self.current_block = span;

        let new_scope = self.scopes.last(span)?.child();
        let scopes_count = self.scopes.push(new_scope);

        for (expr, _) in &block.exprs {
            // NB: terminated expressions do not need to produce a value.
            self.compile_expr(expr, NeedsValue(false))?;
        }

        if let Some(expr) = &block.trailing_expr {
            if !*needs_value {
                self.warnings.not_used(expr.span(), self.context());
            }

            self.compile_expr(expr, needs_value)?;
        }

        let scope = self.scopes.pop(span, scopes_count)?;

        if *needs_value {
            if block.trailing_expr.is_none() {
                self.locals_pop(scope.local_var_count, span);
                self.asm.push(st::Inst::Unit, span);
            } else {
                self.locals_clean(scope.local_var_count, span);
            }
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent context", span))?;
        Ok(())
    }

    /// Encode a return.
    fn compile_expr_return(
        &mut self,
        return_expr: &ast::ExprReturn,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = return_expr.span();
        log::trace!("Return => {:?}", self.source.source(span)?);

        if *needs_value {
            return Err(CompileError::ReturnDoesNotProduceValue {
                block: self.current_block,
                span,
            });
        }

        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let total_var_count = self.scopes.last(span)?.total_var_count;

        if let Some(expr) = &return_expr.expr {
            self.compile_expr(&*expr, NeedsValue(true))?;
            self.locals_clean(total_var_count, span);
            self.asm.push(st::Inst::Return, span);
        } else {
            self.locals_pop(total_var_count, span);
            self.asm.push(st::Inst::ReturnUnit, span);
        }

        Ok(())
    }

    /// Encode an expression.
    fn compile_expr(&mut self, expr: &ast::Expr, needs_value: NeedsValue) -> Result<()> {
        let span = expr.span();
        log::trace!("Expr => {:?}", self.source.source(span)?);

        match expr {
            ast::Expr::ExprWhile(expr_while) => {
                self.compile_expr_while(expr_while, needs_value)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                self.compile_expr_for(expr_for, needs_value)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                self.compile_expr_loop(expr_loop, needs_value)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                self.compile_expr_let(expr_let, needs_value)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.compile_expr(&*expr.expr, needs_value)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.compile_expr_unary(expr_unary, needs_value)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.compile_expr_binary(expr_binary, needs_value)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.compile_expr_if(expr_if, needs_value)?;
            }
            ast::Expr::ExprIndexSet(expr_index_set) => {
                self.compile_index_set(expr_index_set, needs_value)?;
            }
            ast::Expr::ExprIndexGet(expr_index_get) => {
                self.compile_expr_index_get(expr_index_get, needs_value)?;
            }
            ast::Expr::ExprBreak(b) => {
                self.compile_expr_break(b, needs_value)?;
            }
            ast::Expr::ExprBlock(b) => {
                self.compile_expr_block(b, needs_value)?;
            }
            ast::Expr::ExprReturn(return_) => {
                self.compile_expr_return(return_, needs_value)?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                self.compile_expr_match(expr_match, needs_value)?;
            }
            ast::Expr::Ident(ident) => {
                self.compile_ident(ident, needs_value)?;
            }
            ast::Expr::Path(path) => {
                self.compile_path(path, needs_value)?;
            }
            ast::Expr::CallFn(call_fn) => {
                self.compile_call_fn(call_fn, needs_value)?;
            }
            ast::Expr::CallInstanceFn(call_instance_fn) => {
                self.compile_call_instance_fn(call_instance_fn, needs_value)?;
            }
            ast::Expr::LitUnit(lit_unit) => {
                self.compile_lit_unit(lit_unit, needs_value)?;
            }
            ast::Expr::LitBool(lit_bool) => {
                self.compile_lit_bool(lit_bool, needs_value)?;
            }
            ast::Expr::LitNumber(lit_number) => {
                self.compile_lit_number(lit_number, needs_value)?;
            }
            ast::Expr::LitArray(lit_array) => {
                self.compile_lit_array(lit_array, needs_value)?;
            }
            ast::Expr::LitObject(lit_object) => {
                self.compile_lit_object(lit_object, needs_value)?;
            }
            ast::Expr::LitChar(string) => {
                self.compile_lit_char(string, needs_value)?;
            }
            ast::Expr::LitStr(string) => {
                self.compile_lit_str(string, needs_value)?;
            }
        }

        Ok(())
    }

    fn compile_lit_array(
        &mut self,
        lit_array: &ast::LitArray,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = lit_array.span();
        log::trace!("ArrayLiteral => {:?}", self.source.source(span)?);

        if !*needs_value && lit_array.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let count = lit_array.items.len();

        for expr in lit_array.items.iter().rev() {
            self.compile_expr(expr, NeedsValue(true))?;

            // Evaluate the expressions one by one, then pop them to cause any
            // side effects (without creating an object).
            if !*needs_value {
                self.asm.push(st::Inst::Pop, span);
            }
        }

        // No need to create an array if it's not needed.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        self.asm.push(st::Inst::Array { count }, span);
        Ok(())
    }

    fn compile_lit_object(
        &mut self,
        lit_object: &ast::LitObject,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = lit_object.span();
        log::trace!("ObjectLiteral => {:?}", self.source.source(span)?);

        if !*needs_value && lit_object.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let mut keys = Vec::new();
        let mut keys_dup = HashMap::new();

        for (key, _, _) in &lit_object.items {
            let span = key.span();
            let key = key.resolve(self.source)?;
            keys.push(key.to_string());

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: span,
                });
            }
        }

        for (_, _, value) in lit_object.items.iter().rev() {
            self.compile_expr(value, NeedsValue(true))?;

            // Evaluate the expressions one by one, then pop them to cause any
            // side effects (without creating an object).
            if !*needs_value {
                self.asm.push(st::Inst::Pop, span);
            }
        }

        // No need to encode an object since the value is not needed.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let slot = self.unit.new_static_object_keys(&keys)?;

        self.asm.push(st::Inst::Object { slot }, span);
        Ok(())
    }

    /// Encode a char literal, like `'a'`.
    fn compile_lit_char(
        &mut self,
        char_literal: &ast::LitChar,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = char_literal.span();
        log::trace!("CharLiteral => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let resolved_char = char_literal.resolve(self.source)?;
        self.asm.push(st::Inst::Char { c: resolved_char }, span);
        Ok(())
    }

    /// Encode a string literal, like `"foo bar"`.
    fn compile_lit_str(&mut self, string: &ast::LitStr, needs_value: NeedsValue) -> Result<()> {
        let span = string.span();
        log::trace!("StringLiteral => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let string = string.resolve(self.source)?;
        let slot = self.unit.new_static_string(&*string)?;
        self.asm.push(st::Inst::String { slot }, span);
        Ok(())
    }

    fn compile_lit_unit(&mut self, lit_unit: &ast::LitUnit, needs_value: NeedsValue) -> Result<()> {
        let span = lit_unit.span();
        log::trace!("UnitLiteral => {:?}", self.source.source(span)?);

        // If the value is not needed, no need to encode it.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        self.asm.push(st::Inst::Unit, span);
        Ok(())
    }

    fn compile_lit_bool(&mut self, lit_bool: &ast::LitBool, needs_value: NeedsValue) -> Result<()> {
        let span = lit_bool.span();
        log::trace!("BoolLiteral => {:?}", self.source.source(span)?);

        // If the value is not needed, no need to encode it.
        if !*needs_value {
            return Ok(());
        }

        self.asm.push(
            st::Inst::Bool {
                value: lit_bool.value,
            },
            span,
        );
        Ok(())
    }

    /// Compile a literal number.
    fn compile_lit_number(
        &mut self,
        lit_number: &ast::LitNumber,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = lit_number.span();
        log::trace!("NumberLiteral => {:?}", self.source.source(span)?);

        // NB: don't encode unecessary literal.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let lit_number = lit_number.resolve(self.source)?;

        match lit_number {
            ast::Number::Float(number) => {
                self.asm.push(st::Inst::Float { number }, span);
            }
            ast::Number::Integer(number) => {
                self.asm.push(st::Inst::Integer { number }, span);
            }
        }

        Ok(())
    }

    fn compile_expr_while(
        &mut self,
        expr_while: &ast::ExprWhile,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_while.span();
        log::trace!("ExprWhile => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("while_test");
        let end_label = self.asm.new_label("while_end");
        let break_label = self.asm.new_label("while_break");

        let loop_count = self.loops.push(Loop {
            label: expr_while.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.last(span)?.total_var_count,
            needs_value,
        });

        self.asm.label(start_label)?;
        self.compile_expr(&*expr_while.condition, NeedsValue(true))?;
        self.asm.jump_if_not(end_label, span);
        self.compile_expr_block(&*expr_while.body, NeedsValue(false))?;

        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        if *needs_value {
            self.asm.push(st::Inst::Unit, expr_while.condition.span());
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;
        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_for(&mut self, expr_for: &ast::ExprFor, needs_value: NeedsValue) -> Result<()> {
        let span = expr_for.span();
        log::trace!("ExprFor => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("for_start");
        let end_label = self.asm.new_label("for_end");
        let break_label = self.asm.new_label("for_break");

        let new_scope = self.scopes.last(span)?.child();
        let scopes_count = self.scopes.push(new_scope);

        let loop_count = self.loops.push(Loop {
            label: expr_for.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.last(span)?.total_var_count,
            needs_value,
        });

        self.compile_expr(&*expr_for.iter, NeedsValue(true))?;

        // Declare storage for the hidden iterator variable.
        let iterator_offset = self.scopes.last_mut(span)?.decl_anon(expr_for.iter.span());

        // Declare named loop variable.
        let binding_offset = {
            self.asm.push(st::Inst::Unit, expr_for.iter.span());
            let name = expr_for.var.resolve(self.source)?;
            self.scopes
                .last_mut(span)?
                .decl_var(name, expr_for.var.span())
        };

        // Declare storage for memoized `next` instance fn.
        let next_offset = if self.options.memoize_instance_fn {
            let offset = self.scopes.last_mut(span)?.decl_anon(expr_for.iter.span());
            let hash = *st::NEXT;

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

        self.compile_expr_block(&*expr_for.body, NeedsValue(false))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        self.clean_last_scope(span, scopes_count, NeedsValue(false))?;

        // NB: If a value is needed from a for loop, encode it as a unit.
        if *needs_value {
            self.asm.push(st::Inst::Unit, span);
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;

        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_loop(
        &mut self,
        expr_loop: &ast::ExprLoop,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_loop.span();
        log::trace!("ExprLoop => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("loop_start");
        let end_label = self.asm.new_label("loop_end");
        let break_label = self.asm.new_label("loop_break");

        let loop_count = self.loops.push(Loop {
            label: expr_loop.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.last(span)?.total_var_count,
            needs_value,
        });

        self.asm.label(start_label)?;
        self.compile_expr_block(&*expr_loop.body, NeedsValue(false))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // NB: If a value is needed from a while loop, encode it as a unit.
        if *needs_value {
            self.asm.push(st::Inst::Unit, span);
        }

        self.asm.label(break_label)?;
        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_let(&mut self, expr_let: &ast::ExprLet, needs_value: NeedsValue) -> Result<()> {
        let span = expr_let.span();
        log::trace!("ExprLet => {:?}", self.source.source(span)?);

        let false_label = self.asm.new_label("let_panic");
        self.compile_expr(&*expr_let.expr, NeedsValue(true))?;

        let mut scope = self.scopes.pop_unchecked(span)?;

        let load = |_: &mut Assembly| {};

        if self.compile_pat(&mut scope, &expr_let.pat, false_label, &load)? {
            self.warnings.let_pattern_might_panic(span, self.context());

            let ok_label = self.asm.new_label("let_ok");
            self.asm.jump(ok_label, span);
            self.asm.label(false_label)?;
            self.asm.push(
                st::Inst::Panic {
                    reason: st::Panic::UnmatchedPattern,
                },
                span,
            );
            self.asm.label(ok_label)?;
        }

        let _ = self.scopes.push(scope);

        // If a value is needed for a let expression, it is evaluated as a unit.
        if *needs_value {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    fn compile_assign_binop(
        &mut self,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        bin_op: ast::BinOp,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = lhs.span().join(rhs.span());

        let offset = match lhs {
            ast::Expr::Ident(ident) => {
                let name = ident.resolve(self.source)?;
                let var = self.scopes.get_var_mut(name, ident.span())?;
                var.offset
            }
            _ => {
                return Err(CompileError::UnsupportedAssignExpr { span });
            }
        };

        self.compile_expr(rhs, NeedsValue(true))?;

        match bin_op {
            ast::BinOp::Assign => {
                self.asm.push(st::Inst::Replace { offset }, span);
            }
            ast::BinOp::AddAssign => {
                self.asm.push(st::Inst::AddAssign { offset }, span);
            }
            ast::BinOp::SubAssign => {
                self.asm.push(st::Inst::SubAssign { offset }, span);
            }
            ast::BinOp::MulAssign => {
                self.asm.push(st::Inst::MulAssign { offset }, span);
            }
            ast::BinOp::DivAssign => {
                self.asm.push(st::Inst::DivAssign { offset }, span);
            }
            op => {
                return Err(CompileError::UnsupportedAssignBinOp { span, op });
            }
        }

        if *needs_value {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    fn compile_expr_index_get(
        &mut self,
        expr_index_get: &ast::ExprIndexGet,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_index_get.span();
        log::trace!("ExprIndexGet => {:?}", self.source.source(span)?);

        self.compile_expr(&*expr_index_get.index, NeedsValue(true))?;
        self.compile_expr(&*expr_index_get.target, NeedsValue(true))?;
        self.asm.push(st::Inst::IndexGet, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !*needs_value {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a `break` expression.
    fn compile_expr_break(
        &mut self,
        expr_break: &ast::ExprBreak,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_break.span();

        if *needs_value {
            self.warnings
                .break_does_not_produce_value(span, self.context());
        }

        let current_loop = match self.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::BreakOutsideOfLoop { span });
            }
        };

        let (last_loop, has_value) = if let Some(expr) = &expr_break.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    self.compile_expr(&*expr, current_loop.needs_value)?;
                    (current_loop, true)
                }
                ast::ExprBreakValue::Label(label) => {
                    let last_loop = self.loops.walk_until_label(self.source, *label)?;
                    (last_loop, false)
                }
            }
        } else {
            (current_loop, false)
        };

        let vars = self
            .scopes
            .last(span)?
            .total_var_count
            .checked_sub(last_loop.total_var_count)
            .ok_or_else(|| CompileError::internal("var count should be larger", span))?;

        if *last_loop.needs_value {
            if has_value {
                self.locals_clean(vars, span);
            } else {
                self.locals_pop(vars, span);
                self.asm.push(st::Inst::Unit, span);
            }
        } else {
            self.locals_pop(vars, span);
        }

        self.asm.jump(last_loop.break_label, span);
        // NB: loops are expected to produce a value at the end of their expression.
        Ok(())
    }

    fn compile_index_set(
        &mut self,
        expr_index_set: &ast::ExprIndexSet,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_index_set.span();
        log::trace!("ExprIndexSet => {:?}", self.source.source(span)?);

        self.compile_expr(&*expr_index_set.value, NeedsValue(true))?;
        self.compile_expr(&*expr_index_set.index, NeedsValue(true))?;
        self.compile_expr(&*expr_index_set.target, NeedsValue(true))?;
        self.asm.push(st::Inst::IndexSet, span);

        // Encode a unit in case a value is needed.
        if *needs_value {
            self.asm.push(st::Inst::Unit, span);
        }

        Ok(())
    }

    /// Encode a local copy.
    fn compile_ident(&mut self, ident: &ast::Ident, needs_value: NeedsValue) -> Result<()> {
        let span = ident.span();
        log::trace!("Ident => {:?}", self.source.source(span)?);

        // NB: avoid the encode completely if it is not needed.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let target = ident.resolve(self.source)?;
        let var = match self.scopes.get_var(target, span) {
            Ok(var) => var,
            Err(..) => {
                // Something imported is automatically a type.
                if let Some(path) = self.unit.lookup_import_by_name(target) {
                    let hash = st::Hash::of_type(path);
                    self.asm.push(st::Inst::Type { hash }, span);
                    return Ok(());
                }

                return Err(CompileError::MissingLocal {
                    name: target.to_owned(),
                    span,
                });
            }
        };

        self.asm.push(st::Inst::Copy { offset: var.offset }, span);
        Ok(())
    }

    /// Encode the given type.
    fn compile_path(&mut self, path: &ast::Path, needs_value: NeedsValue) -> Result<()> {
        let span = path.span();
        log::trace!("Path => {:?}", self.source.source(span)?);

        // NB: do nothing if we don't need a value.
        if !*needs_value {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let mut parts = Vec::new();
        parts.push(path.first.resolve(self.source)?);

        for (_, part) in &path.rest {
            parts.push(part.resolve(self.source)?);
        }

        let hash = st::Hash::of_type(&parts);
        self.asm.push(st::Inst::Type { hash }, span);
        Ok(())
    }

    fn compile_call_fn(&mut self, call_fn: &ast::CallFn, needs_value: NeedsValue) -> Result<()> {
        let span = call_fn.span();
        log::trace!("CallFn => {:?}", self.source.source(span)?);

        let args = call_fn.args.items.len();

        for (expr, _) in call_fn.args.items.iter().rev() {
            self.compile_expr(expr, NeedsValue(true))?;
        }

        let hash = self.resolve_call_dest(&call_fn.name)?;
        self.asm.push(st::Inst::Call { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !*needs_value {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_call_instance_fn(
        &mut self,
        call_instance_fn: &ast::CallInstanceFn,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = call_instance_fn.span();
        log::trace!("CallInstanceFn => {:?}", self.source.source(span)?);

        let args = call_instance_fn.args.items.len();

        for (expr, _) in call_instance_fn.args.items.iter().rev() {
            self.compile_expr(expr, NeedsValue(true))?;
        }

        self.compile_expr(&*call_instance_fn.instance, NeedsValue(true))?;

        let name = call_instance_fn.name.resolve(self.source)?;
        let hash = st::Hash::of(name);
        self.asm.push(st::Inst::CallInstance { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !*needs_value {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_expr_unary(
        &mut self,
        expr_unary: &ast::ExprUnary,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_unary.span();
        log::trace!("ExprUnary => {:?}", self.source.source(span)?);

        // NB: special unary expressions.
        match expr_unary.op {
            ast::UnaryOp::Ref { .. } => {
                self.compile_ref(&*expr_unary.expr, expr_unary.span(), needs_value)?;
                return Ok(());
            }
            _ => (),
        }

        self.compile_expr(&*expr_unary.expr, NeedsValue(true))?;

        match expr_unary.op {
            ast::UnaryOp::Not { .. } => {
                self.asm.push(st::Inst::Not, span);
            }
            op => {
                return Err(CompileError::UnsupportedUnaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !*needs_value {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a ref `&<expr>` value.
    fn compile_ref(&mut self, expr: &ast::Expr, _: Span, _: NeedsValue) -> Result<()> {
        // TODO: one day this might be supported in one way or another.
        Err(CompileError::UnsupportedRef { span: expr.span() })
    }

    fn compile_expr_binary(
        &mut self,
        expr_binary: &ast::ExprBinary,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_binary.span();
        log::trace!("ExprBinary => {:?}", self.source.source(span)?);

        // Special expressions which operates on the stack in special ways.
        match expr_binary.op {
            ast::BinOp::Assign
            | ast::BinOp::AddAssign
            | ast::BinOp::SubAssign
            | ast::BinOp::MulAssign
            | ast::BinOp::DivAssign => {
                self.compile_assign_binop(
                    &*expr_binary.lhs,
                    &*expr_binary.rhs,
                    expr_binary.op,
                    needs_value,
                )?;
                return Ok(());
            }
            _ => (),
        }

        self.compile_expr(&*expr_binary.rhs, NeedsValue(true))?;
        self.compile_expr(&*expr_binary.lhs, NeedsValue(true))?;

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
            ast::BinOp::And { .. } => {
                self.asm.push(st::Inst::And, span);
            }
            ast::BinOp::Or { .. } => {
                self.asm.push(st::Inst::Or, span);
            }
            op => {
                return Err(CompileError::UnsupportedBinaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !*needs_value {
            self.asm.push(st::Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_condition(
        &mut self,
        condition: &ast::Condition,
        then_label: Label,
    ) -> Result<Scope> {
        let span = condition.span();
        log::trace!("Condition => {:?}", self.source.source(span)?);

        match condition {
            ast::Condition::Expr(expr) => {
                let span = expr.span();

                self.compile_expr(&*expr, NeedsValue(true))?;
                self.asm.jump_if(then_label, span);

                Ok(self.scopes.last(span)?.child())
            }
            ast::Condition::ExprLet(expr_let) => {
                let span = expr_let.span();

                let false_label = self.asm.new_label("if_condition_false");

                let mut scope = self.scopes.last(span)?.child();
                self.compile_expr(&*expr_let.expr, NeedsValue(true))?;

                let load = |_: &mut Assembly| {};

                if self.compile_pat(&mut scope, &expr_let.pat, false_label, &load)? {
                    self.asm.jump(then_label, span);
                    self.asm.label(false_label)?;
                } else {
                    self.asm.jump(then_label, span);
                };

                Ok(scope)
            }
        }
    }

    fn compile_expr_if(&mut self, expr_if: &ast::ExprIf, needs_value: NeedsValue) -> Result<()> {
        let span = expr_if.span();
        log::trace!("ExprIf => {:?}", self.source.source(span)?);

        let then_label = self.asm.new_label("if_then");
        let end_label = self.asm.new_label("if_end");

        let mut branches = Vec::new();
        let then_scope = self.compile_condition(&expr_if.condition, then_label)?;

        for branch in &expr_if.expr_else_ifs {
            let label = self.asm.new_label("if_branch");
            let scope = self.compile_condition(&branch.condition, label)?;
            branches.push((branch, label, scope));
        }

        // use fallback as fall through.
        if let Some(fallback) = &expr_if.expr_else {
            self.compile_expr_block(&*fallback.block, needs_value)?;
        } else {
            // NB: if we must produce a value and there is no fallback branch,
            // encode the result of the statement as a unit.
            if *needs_value {
                self.asm.push(st::Inst::Unit, span);
            }
        }

        self.asm.jump(end_label, span);

        self.asm.label(then_label)?;

        let expected = self.scopes.push(then_scope);
        self.compile_expr_block(&*expr_if.block, needs_value)?;
        self.clean_last_scope(span, expected, needs_value)?;

        if !expr_if.expr_else_ifs.is_empty() {
            self.asm.jump(end_label, span);
        }

        let mut it = branches.into_iter().peekable();

        if let Some((branch, label, scope)) = it.next() {
            let span = branch.span();

            self.asm.label(label)?;

            let scopes = self.scopes.push(scope);
            self.compile_expr_block(&*branch.block, needs_value)?;
            self.clean_last_scope(span, scopes, needs_value)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;
        Ok(())
    }

    fn compile_expr_match(
        &mut self,
        expr_match: &ast::ExprMatch,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let span = expr_match.span();
        log::trace!("ExprMatch => {:?}", self.source.source(span)?);

        let new_scope = self.scopes.last(span)?.child();
        let expected_scopes = self.scopes.push(new_scope);

        self.compile_expr(&*expr_match.expr, NeedsValue(true))?;
        // Offset of the expression.
        let offset = self
            .scopes
            .last_mut(span)?
            .decl_anon(expr_match.expr.span());

        let end_label = self.asm.new_label("match_end");
        let mut branches = Vec::new();

        for (branch, _) in &expr_match.branches {
            let span = branch.span();

            let branch_label = self.asm.new_label("match_branch");
            let match_false = self.asm.new_label("match_false");

            let mut scope = self.scopes.last(span)?.child();

            let load = move |asm: &mut Assembly| {
                asm.push(st::Inst::Copy { offset }, span);
            };

            self.compile_pat(&mut scope, &branch.pat, match_false, &load)?;

            if let Some((_, condition)) = &branch.condition {
                let span = condition.span();
                self.compile_expr(&*condition, NeedsValue(true))?;
                self.asm.jump_if(branch_label, span);
                self.locals_pop(scope.local_var_count, span);
                self.asm.jump(match_false, span);
            }

            self.asm.jump(branch_label, span);
            self.asm.label(match_false)?;

            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if !expr_match.has_default {
            if *needs_value {
                self.asm.push(st::Inst::Unit, span);
            }

            self.asm.jump(end_label, span);
        }

        let mut it = expr_match.branches.iter().zip(&branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            self.asm.label(*label)?;

            let expected = self.scopes.push(scope.clone());
            self.compile_expr(&*branch.body, needs_value)?;
            self.clean_last_scope(span, expected, needs_value)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;

        // pop the implicit scope where we store the anonymous match variable.
        self.clean_last_scope(span, expected_scopes, needs_value)?;
        Ok(())
    }

    /// Encode an array pattern match.
    fn compile_pat_array(
        &mut self,
        scope: &mut Scope,
        array: &ast::PatArray,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = array.span();
        log::trace!("ArrayPat => {:?}", self.source.source(span)?);

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
        self.locals_pop(scope.local_var_count, span);
        self.asm.jump(false_label, span);
        self.asm.label(length_true)?;

        for (index, (pat, _)) in array.items.iter().enumerate() {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                load(asm);
                asm.push(st::Inst::ArrayIndexGet { index }, span);
            };

            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode an object pattern match.
    fn compile_pat_object(
        &mut self,
        scope: &mut Scope,
        object: &ast::PatObject,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = object.span();
        log::trace!("ObjectPat => {:?}", self.source.source(span)?);

        let mut string_slots = Vec::new();

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
        self.locals_pop(scope.local_var_count, span);
        self.asm.jump(false_label, span);
        self.asm.label(length_true)?;

        for ((_, _, pat, _), slot) in object.items.iter().zip(string_slots) {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                load(asm);
                asm.push(st::Inst::ObjectSlotIndexGet { slot }, span);
            };

            // load the given array index and declare it as a local variable.
            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode a pattern.
    ///
    /// Patterns will clean up their own locals and execute a jump to
    /// `false_label` in case the pattern does not match.
    ///
    /// Returns a boolean indicating if the label was used.
    fn compile_pat(
        &mut self,
        scope: &mut Scope,
        pat: &ast::Pat,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<bool> {
        let span = pat.span();
        log::trace!("Pat => {:?}", self.source.source(span)?);

        let true_label = self.asm.new_label("pat_true");

        match pat {
            ast::Pat::PatBinding(binding) => {
                load(&mut self.asm);
                let name = binding.resolve(self.source)?;
                scope.decl_var(name, span);
                return Ok(false);
            }
            ast::Pat::PatIgnore(..) => {
                return Ok(false);
            }
            ast::Pat::PatUnit(unit) => {
                let span = unit.span();

                load(&mut self.asm);
                self.asm.push(st::Inst::IsUnit, span);
                self.asm.jump_if(true_label, span);
            }
            ast::Pat::PatChar(char_literal) => {
                let span = char_literal.span();

                let character = char_literal.resolve(self.source)?;

                load(&mut self.asm);
                self.asm.push(st::Inst::EqCharacter { character }, span);
                self.asm.jump_if(true_label, span);
            }
            ast::Pat::PatNumber(number_literal) => {
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
            ast::Pat::PatString(string) => {
                let span = string.span();

                let string = string.resolve(self.source)?;
                let slot = self.unit.new_static_string(&*string)?;

                load(&mut self.asm);
                self.asm.push(st::Inst::EqStaticString { slot }, span);

                self.asm.jump_if(true_label, span);
            }
            ast::Pat::PatArray(array) => {
                let offset = scope.decl_anon(span);
                load(&mut self.asm);

                let load = |asm: &mut Assembly| {
                    asm.push(st::Inst::Copy { offset }, span);
                };

                self.compile_pat_array(scope, array, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatObject(object) => {
                let offset = scope.decl_anon(span);
                load(&mut self.asm);

                let load = |asm: &mut Assembly| {
                    asm.push(st::Inst::Copy { offset }, span);
                };

                self.compile_pat_object(scope, object, false_label, &load)?;
                return Ok(true);
            }
        }

        // default method of cleaning up locals.
        self.locals_pop(scope.local_var_count, span);
        self.asm.jump(false_label, span);
        self.asm.label(true_label)?;

        Ok(true)
    }

    /// Clean the last scope.
    fn clean_last_scope(
        &mut self,
        span: Span,
        expected: ScopeGuard,
        needs_value: NeedsValue,
    ) -> Result<()> {
        let scope = self.scopes.pop(span, expected)?;

        if *needs_value {
            self.locals_clean(scope.local_var_count, span);
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Decode a path into a call destination based on its hashes.
    fn resolve_call_dest(&self, path: &ast::Path) -> Result<st::Hash> {
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

    /// Get the latest relevant warning context.
    fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }
}
