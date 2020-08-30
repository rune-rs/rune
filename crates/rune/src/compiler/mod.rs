use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use crate::ParseAll;
use runestick::unit::{Assembly, Label, UnitFnCall};
use runestick::{Component, Context, Hash, Inst, Item, Meta, Span, TypeCheck};
use std::cell::RefCell;
use std::rc::Rc;

mod index;
mod items;
mod loops;
mod options;
mod query;
mod scopes;
mod warning;

pub(self) use self::items::Items;
use self::loops::{Loop, Loops};
pub use self::options::Options;
pub use self::query::Query;
use self::scopes::{Scope, ScopeGuard, Scopes};
pub use self::warning::{Warning, Warnings};
use index::{Index as _, Indexer};

type Result<T, E = CompileError> = std::result::Result<T, E>;

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[derive(Debug, Clone, Copy)]
enum Needs {
    Type,
    Value,
    None,
}

impl Needs {
    /// Test if any sort of value is needed.
    fn value(self) -> bool {
        matches!(self, Self::Type | Self::Value)
    }
}

impl<'a> crate::ParseAll<'a, ast::DeclFile> {
    /// Compile the parse with default options.
    pub fn compile(self, context: &Context) -> Result<(runestick::CompilationUnit, Warnings)> {
        self.compile_with_options(context, &Default::default())
    }

    /// Encode the given object into a collection of asm.
    pub fn compile_with_options(
        self,
        context: &Context,
        options: &Options,
    ) -> Result<(runestick::CompilationUnit, Warnings)> {
        let ParseAll { source, item: file } = self;

        let mut warnings = Warnings::new();

        let unit = Rc::new(RefCell::new(
            runestick::CompilationUnit::with_default_prelude(),
        ));

        let mut query = query::Query::new(source, unit.clone());
        let mut indexer = Indexer {
            items: Items::new(vec![]),
            query: &mut query,
            warnings: &mut warnings,
        };
        indexer.index(&file)?;

        while let Some((item, f)) = query.functions.pop_front() {
            let span = f.ast.span();
            let count = f.ast.args.items.len();

            let mut asm = unit.borrow().new_assembly();

            let mut compiler = Compiler {
                context,
                query: &mut query,
                asm: &mut asm,
                items: Items::new(item.as_vec()),
                unit: unit.clone(),
                scopes: Scopes::new(),
                contexts: vec![span],
                source,
                loops: Loops::new(),
                current_block: Span::empty(),
                options,
                warnings: &mut warnings,
            };

            let call = if f.ast.async_.is_some() {
                UnitFnCall::Async
            } else {
                UnitFnCall::Immediate
            };

            compiler.compile_decl_fn(f.ast)?;
            unit.borrow_mut().new_function(item, count, asm, call)?;
        }

        // query holds a reference to the unit, we need to drop it.
        drop(query);

        let unit = Rc::try_unwrap(unit)
            .map_err(|_| CompileError::internal("unit is not exlusively held", Span::empty()))?;

        Ok((unit.into_inner(), warnings))
    }
}

struct Compiler<'a, 'source> {
    /// The context we are compiling for.
    context: &'a Context,
    /// Query system to compile required items.
    query: &'a mut Query<'source>,
    /// The assembly we are generating.
    asm: &'a mut Assembly,
    /// Item builder.
    items: Items,
    /// The compilation unit we are compiling for.
    unit: Rc<RefCell<runestick::CompilationUnit>>,
    /// Scopes defined in the compiler.
    scopes: Scopes,
    /// Context for which to emit warnings.
    contexts: Vec<Span>,
    /// The source we are compiling for.
    source: Source<'source>,
    /// The nesting of loop we are currently in.
    loops: Loops,
    /// The current block that we are in.
    current_block: Span,
    /// Enabled optimizations.
    options: &'a Options,
    /// Compilation warnings.
    warnings: &'a mut Warnings,
}

impl<'a, 'source> Compiler<'a, 'source> {
    fn compile_decl_fn(&mut self, fn_decl: ast::DeclFn) -> Result<()> {
        let span = fn_decl.span();
        log::trace!("FnDecl => {:?}", self.source.source(span)?);
        let item_guard = self.items.push_block();

        for (arg, _) in fn_decl.args.items.iter().rev() {
            let name = arg.resolve(self.source)?;
            self.scopes.last_mut(span)?.new_var(name, arg.span())?;
        }

        if fn_decl.body.exprs.is_empty() && fn_decl.body.trailing_expr.is_none() {
            self.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        for (expr, _) in &fn_decl.body.exprs {
            self.compile_expr(expr, Needs::None)?;
        }

        if let Some(expr) = &fn_decl.body.trailing_expr {
            self.compile_expr(expr, Needs::Value)?;

            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_clean(total_var_count, span);
            self.asm.push(Inst::Return, span);
        } else {
            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
        }

        self.scopes.pop_last(span)?;
        self.items.pop(item_guard, span)?;
        Ok(())
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(&mut self, name: &Item, span: Span) -> Result<Option<Meta>, CompileError> {
        log::trace!("lookup meta: {}", name);

        if let Some(meta) = self.context.lookup_meta(name) {
            log::trace!("found in context: {:?}", meta);
            return Ok(Some(meta));
        }

        let mut base = self.items.item();

        loop {
            let current = base.join(name);
            log::trace!("lookup meta (query): {}", current);

            if let Some(meta) = self.query.query_meta(&current, span)? {
                log::trace!("found in query: {:?}", meta);
                return Ok(Some(meta));
            }

            if base.pop().is_none() {
                break;
            }
        }

        Ok(None)
    }

    /// Pop locals by simply popping them.
    fn locals_pop(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(Inst::Pop, span);
            }
            count => {
                self.asm.push(Inst::PopN { count }, span);
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
                self.asm.push(Inst::Clean { count }, span);
            }
        }
    }

    /// Encode a block.
    ///
    /// Blocks are special in that they do not produce a value unless there is
    /// an item in them which does.
    fn compile_expr_block(&mut self, block: &ast::ExprBlock, needs: Needs) -> Result<()> {
        let span = block.span();
        log::trace!("ExprBlock => {:?}", self.source.source(span)?);
        let item_guard = self.items.push_block();

        self.contexts.push(span);

        let span = block.span();
        self.current_block = span;

        let new_scope = self.scopes.last(span)?.child();
        let scopes_count = self.scopes.push(new_scope);

        for (expr, _) in &block.exprs {
            // NB: terminated expressions do not need to produce a value.
            self.compile_expr(expr, Needs::None)?;
        }

        if let Some(expr) = &block.trailing_expr {
            self.compile_expr(expr, needs)?;
        }

        let scope = self.scopes.pop(span, scopes_count)?;

        if needs.value() {
            if block.trailing_expr.is_none() {
                self.locals_pop(scope.local_var_count, span);
                self.asm.push(Inst::Unit, span);
            } else {
                self.locals_clean(scope.local_var_count, span);
            }
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent context", span))?;

        self.items.pop(item_guard, span)?;
        Ok(())
    }

    /// Encode a return.
    fn compile_expr_return(&mut self, return_expr: &ast::ExprReturn, _needs: Needs) -> Result<()> {
        let span = return_expr.span();
        log::trace!("Return => {:?}", self.source.source(span)?);

        // NB: drop any loop temporaries.
        for l in &self.loops {
            if let Some(offset) = l.drop {
                self.asm.push(Inst::Drop { offset }, span);
            }
        }

        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let total_var_count = self.scopes.last(span)?.total_var_count;

        if let Some(expr) = &return_expr.expr {
            self.compile_expr(&*expr, Needs::Value)?;
            self.locals_clean(total_var_count, span);
            self.asm.push(Inst::Return, span);
        } else {
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
        }

        Ok(())
    }

    /// Encode an expression.
    fn compile_expr(&mut self, expr: &ast::Expr, needs: Needs) -> Result<()> {
        let span = expr.span();
        log::trace!("Expr => {:?}", self.source.source(span)?);

        match expr {
            ast::Expr::ExprWhile(expr_while) => {
                self.compile_expr_while(expr_while, needs)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                self.compile_expr_for(expr_for, needs)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                self.compile_expr_loop(expr_loop, needs)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                self.compile_expr_let(expr_let, needs)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.compile_expr(&*expr.expr, needs)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.compile_expr_unary(expr_unary, needs)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.compile_expr_binary(expr_binary, needs)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.compile_expr_if(expr_if, needs)?;
            }
            ast::Expr::ExprIndexSet(expr_index_set) => {
                self.compile_index_set(expr_index_set, needs)?;
            }
            ast::Expr::ExprIndexGet(expr_index_get) => {
                self.compile_expr_index_get(expr_index_get, needs)?;
            }
            ast::Expr::ExprBreak(expr_break) => {
                self.compile_expr_break(expr_break, needs)?;
            }
            ast::Expr::ExprBlock(expr_block) => {
                self.compile_expr_block(expr_block, needs)?;
            }
            ast::Expr::ExprReturn(expr_return) => {
                self.compile_expr_return(expr_return, needs)?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                self.compile_expr_match(expr_match, needs)?;
            }
            ast::Expr::ExprAwait(expr_await) => {
                self.compile_expr_await(expr_await, needs)?;
            }
            ast::Expr::ExprTry(expr_try) => {
                self.compile_expr_try(expr_try, needs)?;
            }
            ast::Expr::ExprSelect(expr_select) => {
                self.compile_expr_select(expr_select, needs)?;
            }
            ast::Expr::Path(path) => {
                self.compile_path(path, needs)?;
            }
            ast::Expr::CallFn(call_fn) => {
                self.compile_call_fn(call_fn, needs)?;
            }
            ast::Expr::CallInstanceFn(call_instance_fn) => {
                self.compile_call_instance_fn(call_instance_fn, needs)?;
            }
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                self.compile_expr_field_access(expr_field_access, needs)?;
            }
            ast::Expr::LitUnit(lit_unit) => {
                self.compile_lit_unit(lit_unit, needs)?;
            }
            ast::Expr::LitTuple(lit_tuple) => {
                self.compile_lit_tuple(lit_tuple, needs)?;
            }
            ast::Expr::LitBool(lit_bool) => {
                self.compile_lit_bool(lit_bool, needs)?;
            }
            ast::Expr::LitNumber(lit_number) => {
                self.compile_lit_number(lit_number, needs)?;
            }
            ast::Expr::LitVec(lit_vec) => {
                self.compile_lit_vec(lit_vec, needs)?;
            }
            ast::Expr::LitObject(lit_object) => {
                self.compile_lit_object(lit_object, needs)?;
            }
            ast::Expr::LitChar(lit_char) => {
                self.compile_lit_char(lit_char, needs)?;
            }
            ast::Expr::LitStr(lit_str) => {
                self.compile_lit_str(lit_str, needs)?;
            }
            ast::Expr::LitByte(lit_char) => {
                self.compile_lit_byte(lit_char, needs)?;
            }
            ast::Expr::LitByteStr(lit_str) => {
                self.compile_lit_byte_str(lit_str, needs)?;
            }
            ast::Expr::LitTemplate(lit_template) => {
                self.compile_lit_template(lit_template, needs)?;
            }
            // NB: declarations are not used in this compilation stage.
            // They have been separately indexed and will be built when queried
            // for.
            ast::Expr::Decl(decl) => {
                let span = decl.span();

                if needs.value() {
                    self.asm.push(Inst::Unit, span);
                }
            }
        }

        Ok(())
    }

    fn compile_lit_vec(&mut self, lit_vec: &ast::LitVec, needs: Needs) -> Result<()> {
        let span = lit_vec.span();
        log::trace!("LitVec => {:?}", self.source.source(span)?);

        if !needs.value() && lit_vec.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let count = lit_vec.items.len();

        for expr in lit_vec.items.iter().rev() {
            self.compile_expr(expr, Needs::Value)?;

            // Evaluate the expressions one by one, then pop them to cause any
            // side effects (without creating an object).
            if !needs.value() {
                self.asm.push(Inst::Pop, span);
            }
        }

        // No need to create a vector if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        self.asm.push(Inst::Vec { count }, span);
        Ok(())
    }

    fn compile_lit_object(&mut self, lit_object: &ast::LitObject, needs: Needs) -> Result<()> {
        let span = lit_object.span();
        log::trace!("LitObject => {:?}", self.source.source(span)?);

        if !needs.value() && lit_object.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let mut keys = Vec::new();
        let mut check_keys = Vec::new();
        let mut keys_dup = HashMap::new();

        for assign in &lit_object.assignments {
            let span = assign.span();
            let key = assign.key.resolve(self.source)?;
            keys.push(key.to_string());
            check_keys.push((key.to_string(), assign.key.span()));

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: span,
                });
            }
        }

        for assign in lit_object.assignments.iter().rev() {
            let span = assign.span();

            if let Some((_, expr)) = &assign.assign {
                self.compile_expr(expr, Needs::Value)?;

                // Evaluate the expressions one by one, then pop them to cause any
                // side effects (without creating an object).
                if !needs.value() {
                    self.asm.push(Inst::Pop, span);
                }
            } else {
                let key = assign.key.resolve(self.source)?;
                let var = self.scopes.get_var(&*key, span)?;

                if needs.value() {
                    self.asm.push(Inst::Copy { offset: var.offset }, span);
                }
            }
        }

        // No need to encode an object since the value is not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let slot = self.unit.borrow_mut().new_static_object_keys(&keys)?;

        match &lit_object.ident {
            ast::LitObjectIdent::Named(path) => {
                let item = self.convert_path_to_item(path)?;

                let meta = match self.lookup_meta(&item, path.span())? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::MissingType { span, item });
                    }
                };

                match meta {
                    Meta::MetaObject { object } => {
                        check_object_fields(
                            object.fields.as_ref(),
                            check_keys,
                            span,
                            &object.item,
                        )?;

                        let hash = Hash::type_hash(&object.item);
                        self.asm.push(Inst::TypedObject { hash, slot }, span);
                    }
                    Meta::MetaObjectVariant { enum_item, object } => {
                        check_object_fields(
                            object.fields.as_ref(),
                            check_keys,
                            span,
                            &object.item,
                        )?;

                        let enum_hash = Hash::type_hash(&enum_item);
                        let hash = Hash::type_hash(&object.item);

                        self.asm.push(
                            Inst::VariantObject {
                                enum_hash,
                                hash,
                                slot,
                            },
                            span,
                        );
                    }
                    meta => {
                        return Err(CompileError::UnsupportedLitObject {
                            span,
                            item: meta.item().clone(),
                        });
                    }
                };
            }
            ast::LitObjectIdent::Anonymous(..) => {
                self.asm.push(Inst::Object { slot }, span);
            }
        }

        return Ok(());

        fn check_object_fields(
            fields: Option<&HashSet<String>>,
            check_keys: Vec<(String, Span)>,
            span: Span,
            item: &Item,
        ) -> Result<(), CompileError> {
            let mut fields = match fields {
                Some(fields) => fields.clone(),
                None => {
                    return Err(CompileError::MissingType {
                        span,
                        item: item.clone(),
                    });
                }
            };

            for (field, span) in check_keys {
                if !fields.remove(&field) {
                    return Err(CompileError::LitObjectNotField {
                        span,
                        field,
                        item: item.clone(),
                    });
                }
            }

            if let Some(field) = fields.into_iter().next() {
                return Err(CompileError::LitObjectMissingField {
                    span,
                    field,
                    item: item.clone(),
                });
            }

            Ok(())
        }
    }

    /// Encode a char literal, like `'a'`.
    fn compile_lit_char(&mut self, lit_char: &ast::LitChar, needs: Needs) -> Result<()> {
        let span = lit_char.span();
        log::trace!("LitChar => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let resolved_char = lit_char.resolve(self.source)?;
        self.asm.push(Inst::Char { c: resolved_char }, span);
        Ok(())
    }

    /// Encode a string literal, like `"foo bar"`.
    fn compile_lit_str(&mut self, lit_str: &ast::LitStr, needs: Needs) -> Result<()> {
        let span = lit_str.span();
        log::trace!("LitStr => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let string = lit_str.resolve(self.source)?;
        let slot = self.unit.borrow_mut().new_static_string(&*string)?;
        self.asm.push(Inst::String { slot }, span);
        Ok(())
    }

    /// Encode a byte literal, like `b'a'`.
    fn compile_lit_byte(&mut self, lit_byte: &ast::LitByte, needs: Needs) -> Result<()> {
        let span = lit_byte.span();
        log::trace!("LitByte => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let b = lit_byte.resolve(self.source)?;
        self.asm.push(Inst::Byte { b }, span);
        Ok(())
    }

    /// Encode a byte string literal, like `b"foo bar"`.
    fn compile_lit_byte_str(&mut self, lit_byte_str: &ast::LitByteStr, needs: Needs) -> Result<()> {
        let span = lit_byte_str.span();
        log::trace!("LitByteStr => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let bytes = lit_byte_str.resolve(self.source)?;
        let slot = self.unit.borrow_mut().new_static_bytes(&*bytes)?;
        self.asm.push(Inst::Bytes { slot }, span);
        Ok(())
    }

    /// Encode a string literal, like `"foo bar"`.
    fn compile_lit_template(
        &mut self,
        lit_template: &ast::LitTemplate,
        needs: Needs,
    ) -> Result<()> {
        let span = lit_template.span();
        log::trace!("LitTemplate => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let template = lit_template.resolve(self.source)?;

        if !template.has_expansions {
            self.warnings
                .template_without_expansions(span, self.context());
        }

        let scope = self.scopes.last(span)?.child();
        let expected = self.scopes.push(scope);

        for c in template.components.iter().rev() {
            match c {
                ast::TemplateComponent::String(string) => {
                    let slot = self.unit.borrow_mut().new_static_string(&string)?;
                    self.asm.push(Inst::String { slot }, span);
                    self.scopes.last_mut(span)?.decl_anon(span);
                }
                ast::TemplateComponent::Expr(expr) => {
                    self.compile_expr(expr, Needs::Value)?;
                    self.scopes.last_mut(span)?.decl_anon(span);
                }
            }
        }

        self.asm.push(
            Inst::StringConcat {
                len: template.components.len(),
                size_hint: template.size_hint,
            },
            span,
        );

        let _ = self.scopes.pop(span, expected)?;
        Ok(())
    }

    fn compile_lit_unit(&mut self, lit_unit: &ast::LitUnit, needs: Needs) -> Result<()> {
        let span = lit_unit.span();
        log::trace!("LitUnit => {:?}", self.source.source(span)?);

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            return Ok(());
        }

        self.asm.push(Inst::Unit, span);
        Ok(())
    }

    fn compile_lit_tuple(&mut self, lit_tuple: &ast::LitTuple, needs: Needs) -> Result<()> {
        let span = lit_tuple.span();
        log::trace!("LitTuple => {:?}", self.source.source(span)?);

        // If the value is not needed, no need to encode it.
        if !needs.value() && lit_tuple.is_const() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        for (expr, _) in lit_tuple.items.iter().rev() {
            self.compile_expr(expr, Needs::Value)?;
        }

        self.asm.push(
            Inst::Tuple {
                count: lit_tuple.items.len(),
            },
            span,
        );

        Ok(())
    }

    fn compile_lit_bool(&mut self, lit_bool: &ast::LitBool, needs: Needs) -> Result<()> {
        let span = lit_bool.span();
        log::trace!("LitBool => {:?}", self.source.source(span)?);

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            return Ok(());
        }

        self.asm.push(
            Inst::Bool {
                value: lit_bool.value,
            },
            span,
        );
        Ok(())
    }

    /// Compile a literal number.
    fn compile_lit_number(&mut self, lit_number: &ast::LitNumber, needs: Needs) -> Result<()> {
        let span = lit_number.span();
        log::trace!("LitNumber => {:?}", self.source.source(span)?);

        // NB: don't encode unecessary literal.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let lit_number = lit_number.resolve(self.source)?;

        match lit_number {
            ast::Number::Float(number) => {
                self.asm.push(Inst::Float { number }, span);
            }
            ast::Number::Integer(number) => {
                self.asm.push(Inst::Integer { number }, span);
            }
        }

        Ok(())
    }

    fn compile_expr_while(&mut self, expr_while: &ast::ExprWhile, needs: Needs) -> Result<()> {
        let span = expr_while.span();
        log::trace!("ExprWhile => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("while_test");
        let then_label = self.asm.new_label("while_then");
        let end_label = self.asm.new_label("while_end");
        let break_label = self.asm.new_label("while_break");

        let loop_count = self.loops.push(Loop {
            label: expr_while.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.last(span)?.total_var_count,
            needs,
            drop: None,
        });

        self.asm.label(start_label)?;

        let then_scope = self.compile_condition(&expr_while.condition, then_label)?;
        self.asm.jump(end_label, span);
        self.asm.label(then_label)?;

        let expected = self.scopes.push(then_scope);
        self.compile_expr_block(&*expr_while.body, Needs::None)?;
        self.clean_last_scope(span, expected, Needs::None)?;

        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;
        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_for(&mut self, expr_for: &ast::ExprFor, needs: Needs) -> Result<()> {
        let span = expr_for.span();
        log::trace!("ExprFor => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("for_start");
        let end_label = self.asm.new_label("for_end");
        let break_label = self.asm.new_label("for_break");

        let total_var_count = self.scopes.last(span)?.total_var_count;

        let (iter_offset, loop_scope_expected) = {
            let mut loop_scope = self.scopes.last(span)?.child();
            self.compile_expr(&*expr_for.iter, Needs::Value)?;
            self.asm.push(
                Inst::CallInstance {
                    hash: *runestick::INTO_ITER,
                    args: 0,
                },
                span,
            );
            let iter_offset = loop_scope.decl_anon(span);
            let loop_scope_expected = self.scopes.push(loop_scope);
            (iter_offset, loop_scope_expected)
        };

        let loop_count = self.loops.push(Loop {
            label: expr_for.label.map(|(label, _)| label),
            break_label,
            total_var_count,
            needs,
            drop: Some(iter_offset),
        });

        // Declare named loop variable.
        let binding_offset = {
            self.asm.push(Inst::Unit, expr_for.iter.span());
            let name = expr_for.var.resolve(self.source)?;
            self.scopes
                .last_mut(span)?
                .decl_var(name, expr_for.var.span())
        };

        // Declare storage for memoized `next` instance fn.
        let next_offset = if self.options.memoize_instance_fn {
            let span = expr_for.iter.span();

            let offset = self.scopes.last_mut(span)?.decl_anon(span);
            let hash = *runestick::NEXT;

            // Declare the named loop variable and put it in the scope.
            self.asm.push(
                Inst::Copy {
                    offset: iter_offset,
                },
                span,
            );

            self.asm.push(Inst::LoadInstanceFn { hash }, span);
            Some(offset)
        } else {
            None
        };

        self.asm.label(start_label)?;

        // Use the memoized loop variable.
        if let Some(next_offset) = next_offset {
            self.asm.push(
                Inst::Copy {
                    offset: iter_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push(
                Inst::Copy {
                    offset: next_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push(Inst::CallFn { args: 0 }, span);

            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        } else {
            // call the `next` function to get the next level of iteration, bind the
            // result to the loop variable in the loop.
            self.asm.push(
                Inst::Copy {
                    offset: iter_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push(
                Inst::CallInstance {
                    hash: *runestick::NEXT,
                    args: 0,
                },
                span,
            );
            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        }

        // test loop condition and unwrap the option.
        // TODO: introduce a dedicated instruction for this :|.
        {
            self.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
            self.asm.push(Inst::IsValue, expr_for.span());
            self.asm.jump_if_not(end_label, expr_for.span());
            self.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
            // unwrap the optional value.
            self.asm.push(Inst::Unwrap, expr_for.span());
            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        }

        self.compile_expr_block(&*expr_for.body, Needs::None)?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // Drop the iterator.
        self.asm.push(
            Inst::Drop {
                offset: iter_offset,
            },
            span,
        );

        self.clean_last_scope(span, loop_scope_expected, Needs::None)?;

        // NB: If a value is needed from a for loop, encode it as a unit.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;
        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_loop(&mut self, expr_loop: &ast::ExprLoop, needs: Needs) -> Result<()> {
        let span = expr_loop.span();
        log::trace!("ExprLoop => {:?}", self.source.source(span)?);

        let start_label = self.asm.new_label("loop_start");
        let end_label = self.asm.new_label("loop_end");
        let break_label = self.asm.new_label("loop_break");

        let loop_count = self.loops.push(Loop {
            label: expr_loop.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.last(span)?.total_var_count,
            needs,
            drop: None,
        });

        self.asm.label(start_label)?;
        self.compile_expr_block(&*expr_loop.body, Needs::None)?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // NB: If a value is needed from a while loop, encode it as a unit.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        self.asm.label(break_label)?;
        self.loops.pop(span, loop_count)?;
        Ok(())
    }

    fn compile_expr_let(&mut self, expr_let: &ast::ExprLet, needs: Needs) -> Result<()> {
        let span = expr_let.span();
        log::trace!("ExprLet => {:?}", self.source.source(span)?);

        // NB: assignments "move" the value being assigned.
        self.compile_expr(&*expr_let.expr, Needs::Value)?;

        let mut scope = self.scopes.pop_unchecked(span)?;

        let load = |_: &mut Assembly| {};

        let false_label = self.asm.new_label("let_panic");

        if self.compile_pat(&mut scope, &expr_let.pat, false_label, &load)? {
            self.warnings.let_pattern_might_panic(span, self.context());

            let ok_label = self.asm.new_label("let_ok");
            self.asm.jump(ok_label, span);
            self.asm.label(false_label)?;
            self.asm.push(
                Inst::Panic {
                    reason: runestick::PanicReason::UnmatchedPattern,
                },
                span,
            );
            self.asm.label(ok_label)?;
        }

        let _ = self.scopes.push(scope);

        // If a value is needed for a let expression, it is evaluated as a unit.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        Ok(())
    }

    fn compile_assign_binop(
        &mut self,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        bin_op: ast::BinOp,
        needs: Needs,
    ) -> Result<()> {
        let span = lhs.span().join(rhs.span());

        // NB: this loop is actually useful in breaking early.
        #[allow(clippy::never_loop)]
        let offset = loop {
            match lhs {
                ast::Expr::Path(path) => {
                    let item = self.convert_path_to_item(path)?;

                    if let Some(local) = item.as_local() {
                        let var = self.scopes.get_var(local, path.span())?;
                        break var.offset;
                    }
                }
                ast::Expr::ExprBinary(expr_binary) => {
                    if let (ast::Expr::Path(var), ast::Expr::Path(field)) =
                        (&*expr_binary.lhs, &*expr_binary.rhs)
                    {
                        let field_span = field.span();
                        let var_span = var.span();

                        let var = self.convert_path_to_item(var)?;
                        let field = self.convert_path_to_item(field)?;

                        if let (Some(var), Some(field)) = (var.as_local(), field.as_local()) {
                            self.compile_expr(rhs, Needs::Value)?;

                            let field = self.unit.borrow_mut().new_static_string(field)?;
                            self.asm.push(Inst::String { slot: field }, field_span);

                            let var = self.scopes.get_var(var, var_span)?.offset;
                            self.asm.push(Inst::Copy { offset: var }, var_span);

                            self.asm.push(Inst::IndexSet, span);
                            return Ok(());
                        }
                    }
                }
                _ => (),
            };

            return Err(CompileError::UnsupportedAssignExpr { span });
        };

        self.compile_expr(rhs, Needs::Value)?;

        match bin_op {
            ast::BinOp::Assign => {
                self.asm.push(Inst::Replace { offset }, span);
            }
            ast::BinOp::AddAssign => {
                self.asm.push(Inst::AddAssign { offset }, span);
            }
            ast::BinOp::SubAssign => {
                self.asm.push(Inst::SubAssign { offset }, span);
            }
            ast::BinOp::MulAssign => {
                self.asm.push(Inst::MulAssign { offset }, span);
            }
            ast::BinOp::DivAssign => {
                self.asm.push(Inst::DivAssign { offset }, span);
            }
            op => {
                return Err(CompileError::UnsupportedAssignBinOp { span, op });
            }
        }

        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        Ok(())
    }

    /// Compile field access for the given expression.
    fn compile_expr_field_access(
        &mut self,
        expr_field_access: &ast::ExprFieldAccess,
        needs: Needs,
    ) -> Result<()> {
        use std::convert::TryFrom as _;

        let span = expr_field_access.span();

        // Optimizations!
        //
        // TODO: perform deferred compilation for expressions instead, so we can
        // e.g. inspect if it compiles down to a local access instead of
        // climbing the ast like we do here.
        #[allow(clippy::single_match)]
        match (&*expr_field_access.expr, &expr_field_access.expr_field) {
            (ast::Expr::Path(path), ast::ExprField::LitNumber(n)) => {
                if try_immediate_field_access_optimization(self, span, path, n, needs)? {
                    return Ok(());
                }
            }
            _ => (),
        }

        self.compile_expr(&*expr_field_access.expr, Needs::Value)?;

        // This loop is actually useful.
        #[allow(clippy::never_loop)]
        loop {
            match &expr_field_access.expr_field {
                ast::ExprField::LitNumber(n) => {
                    let index = match n.resolve(self.source)? {
                        ast::Number::Integer(n) if n >= 0 => match usize::try_from(n) {
                            Ok(n) => n,
                            Err(..) => break,
                        },
                        _ => break,
                    };

                    self.asm.push(Inst::TupleIndexGet { index }, span);

                    if !needs.value() {
                        self.warnings.not_used(span, self.context());
                        self.asm.push(Inst::Pop, span);
                    }

                    return Ok(());
                }
                ast::ExprField::Ident(ident) => {
                    let field = ident.resolve(self.source)?;
                    let slot = self.unit.borrow_mut().new_static_string(field)?;

                    self.asm.push(Inst::ObjectSlotIndexGet { slot }, span);

                    if !needs.value() {
                        self.warnings.not_used(span, self.context());
                        self.asm.push(Inst::Pop, span);
                    }

                    return Ok(());
                }
            }
        }

        return Err(CompileError::UnsupportedFieldAccess { span });

        fn try_immediate_field_access_optimization(
            this: &mut Compiler<'_, '_>,
            span: Span,
            path: &ast::Path,
            n: &ast::LitNumber,
            needs: Needs,
        ) -> Result<bool, CompileError> {
            let ident = match path.try_as_ident() {
                Some(ident) => ident,
                None => return Ok(false),
            };

            let ident = ident.resolve(this.source)?;

            let index = match n.resolve(this.source)? {
                ast::Number::Integer(n) => n,
                _ => return Ok(false),
            };

            let index = match usize::try_from(index) {
                Ok(index) => index,
                Err(..) => return Ok(false),
            };

            let var = match this.scopes.try_get_var(ident)? {
                Some(var) => var,
                None => return Ok(false),
            };

            this.asm.push(
                Inst::TupleIndexGetAt {
                    offset: var.offset,
                    index,
                },
                span,
            );

            if !needs.value() {
                this.warnings.not_used(span, this.context());
                this.asm.push(Inst::Pop, span);
            }

            Ok(true)
        }
    }

    fn compile_expr_index_get(
        &mut self,
        expr_index_get: &ast::ExprIndexGet,
        needs: Needs,
    ) -> Result<()> {
        let span = expr_index_get.span();
        log::trace!("ExprIndexGet => {:?}", self.source.source(span)?);

        self.compile_expr(&*expr_index_get.index, Needs::Value)?;
        self.compile_expr(&*expr_index_get.target, Needs::Value)?;
        self.asm.push(Inst::IndexGet, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a `break` expression.
    fn compile_expr_break(&mut self, expr_break: &ast::ExprBreak, needs: Needs) -> Result<()> {
        let span = expr_break.span();

        if needs.value() {
            self.warnings
                .break_does_not_produce_value(span, self.context());
        }

        let current_loop = match self.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::BreakOutsideOfLoop { span });
            }
        };

        let (last_loop, to_drop, has_value) = if let Some(expr) = &expr_break.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    self.compile_expr(&*expr, current_loop.needs)?;
                    (current_loop, current_loop.drop.into_iter().collect(), true)
                }
                ast::ExprBreakValue::Label(label) => {
                    let (last_loop, to_drop) = self.loops.walk_until_label(self.source, *label)?;
                    (last_loop, to_drop, false)
                }
            }
        } else {
            (current_loop, current_loop.drop.into_iter().collect(), false)
        };

        // Drop loop temporary. Typically an iterator.
        for offset in to_drop {
            self.asm.push(Inst::Drop { offset }, span);
        }

        let vars = self
            .scopes
            .last(span)?
            .total_var_count
            .checked_sub(last_loop.total_var_count)
            .ok_or_else(|| CompileError::internal("var count should be larger", span))?;

        if last_loop.needs.value() {
            if has_value {
                self.locals_clean(vars, span);
            } else {
                self.locals_pop(vars, span);
                self.asm.push(Inst::Unit, span);
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
        needs: Needs,
    ) -> Result<()> {
        let span = expr_index_set.span();
        log::trace!("ExprIndexSet => {:?}", self.source.source(span)?);

        self.compile_expr(&*expr_index_set.value, Needs::Value)?;
        self.compile_expr(&*expr_index_set.index, Needs::Value)?;
        self.compile_expr(&*expr_index_set.target, Needs::Value)?;
        self.asm.push(Inst::IndexSet, span);

        // Encode a unit in case a value is needed.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        Ok(())
    }

    /// Compile an item.
    fn compile_meta(&mut self, meta: &Meta, span: Span, needs: Needs) -> Result<()> {
        log::trace!("Meta => {:?} {:?}", meta, needs);

        match (needs, meta) {
            (Needs::Value, Meta::MetaTuple { tuple }) if tuple.args == 0 => {
                let hash = Hash::type_hash(&tuple.item);
                self.asm.push(Inst::Call { hash, args: 0 }, span);
            }
            (Needs::Value, Meta::MetaTupleVariant { tuple, .. }) if tuple.args == 0 => {
                let hash = Hash::type_hash(&tuple.item);
                self.asm.push(Inst::Call { hash, args: 0 }, span);
            }
            (Needs::Value, Meta::MetaTuple { tuple }) => {
                let hash = Hash::type_hash(&tuple.item);
                self.asm.push(Inst::Fn { hash }, span);
            }
            (Needs::Value, Meta::MetaTupleVariant { tuple, .. }) => {
                let hash = Hash::type_hash(&tuple.item);
                self.asm.push(Inst::Fn { hash }, span);
            }
            (Needs::Value, Meta::MetaFunction { item }) => {
                let hash = Hash::type_hash(item);
                self.asm.push(Inst::Fn { hash }, span);
            }
            (_, meta) => {
                let hash = Hash::type_hash(meta.item());
                self.asm.push(Inst::Type { hash }, span);
            }
        }

        Ok(())
    }

    /// Encode the given type.
    fn compile_path(&mut self, path: &ast::Path, needs: Needs) -> Result<()> {
        let span = path.span();
        log::trace!("Path => {:?}", self.source.source(span)?);

        // NB: do nothing if we don't need a value.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let item = self.convert_path_to_item(path)?;

        if let Needs::Value = needs {
            if let Some(local) = item.as_local() {
                if let Some(var) = self.scopes.try_get_var(local)? {
                    self.asm.push(Inst::Copy { offset: var.offset }, span);
                    return Ok(());
                }
            }
        }

        let meta = match self.lookup_meta(&item, span)? {
            Some(meta) => meta,
            None => match (needs, item.as_local()) {
                (Needs::Value, Some(local)) => {
                    return Err(CompileError::MissingLocal {
                        name: local.to_owned(),
                        span,
                    });
                }
                _ => {
                    return Err(CompileError::MissingType { span, item });
                }
            },
        };

        self.compile_meta(&meta, span, needs)?;
        Ok(())
    }

    /// Lookup the given local name.
    fn lookup_import_by_name(&self, local: &Component) -> Option<Item> {
        let unit = self.unit.borrow();

        let mut base = self.items.item();

        loop {
            if let Some(item) = unit.lookup_import_by_name(&base, &local).cloned() {
                return Some(item);
            }

            if base.pop().is_none() {
                break;
            }
        }

        None
    }

    /// Convert a path to an item.
    fn convert_path_to_item(&self, path: &ast::Path) -> Result<Item> {
        let local = Component::from(path.first.resolve(self.source)?);

        let imported = match self.lookup_import_by_name(&local) {
            Some(path) => path,
            None => Item::of(&[local]),
        };

        let mut rest = Vec::new();

        for (_, part) in &path.rest {
            rest.push(Component::String(part.resolve(self.source)?.to_owned()));
        }

        let it = imported.into_iter().chain(rest.into_iter());
        Ok(Item::of(it))
    }

    fn compile_call_fn(&mut self, call_fn: &ast::CallFn, needs: Needs) -> Result<()> {
        let span = call_fn.span();
        log::trace!("CallFn => {:?}", self.source.source(span)?);

        let args = call_fn.args.items.len();

        for (expr, _) in call_fn.args.items.iter().rev() {
            self.compile_expr(expr, Needs::Value)?;
        }

        let item = self.convert_path_to_item(&call_fn.name)?;
        let a = self.lookup_meta(&item, call_fn.name.span())?;

        let item = if let Some(meta) = a {
            match &meta {
                Meta::MetaTuple { tuple } | Meta::MetaTupleVariant { tuple, .. } => {
                    if tuple.args != call_fn.args.items.len() {
                        return Err(CompileError::UnsupportedArgumentCount {
                            span,
                            meta: meta.clone(),
                            expected: tuple.args,
                            actual: call_fn.args.items.len(),
                        });
                    }

                    if tuple.args == 0 {
                        let tuple = call_fn.name.span();
                        self.warnings
                            .remove_tuple_call_parens(span, tuple, self.context());
                    }

                    tuple.item.clone()
                }
                Meta::MetaFunction { item } => item.clone(),
                _ => {
                    return Err(CompileError::NotFunction { span });
                }
            }
        } else {
            item
        };

        if let Some(name) = item.as_local() {
            if let Some(var) = self.scopes.last(span)?.get(name) {
                self.asm.push(Inst::Copy { offset: var.offset }, span);
                self.asm.push(Inst::CallFn { args }, span);
                return Ok(());
            }
        }

        let hash = Hash::type_hash(&item);
        self.asm.push(Inst::Call { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_call_instance_fn(
        &mut self,
        call_instance_fn: &ast::CallInstanceFn,
        needs: Needs,
    ) -> Result<()> {
        let span = call_instance_fn.span();
        log::trace!("CallInstanceFn => {:?}", self.source.source(span)?);

        let args = call_instance_fn.args.items.len();

        for (expr, _) in call_instance_fn.args.items.iter().rev() {
            self.compile_expr(expr, Needs::Value)?;
        }

        self.compile_expr(&*call_instance_fn.instance, Needs::Value)?;

        let name = call_instance_fn.name.resolve(self.source)?;
        let hash = Hash::of(name);
        self.asm.push(Inst::CallInstance { hash, args }, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_expr_unary(&mut self, expr_unary: &ast::ExprUnary, needs: Needs) -> Result<()> {
        let span = expr_unary.span();
        log::trace!("ExprUnary => {:?}", self.source.source(span)?);

        // NB: special unary expressions.
        if let ast::UnaryOp::BorrowRef { .. } = expr_unary.op {
            self.compile_ref(&*expr_unary.expr, expr_unary.span(), needs)?;
            return Ok(());
        }

        self.compile_expr(&*expr_unary.expr, Needs::Value)?;

        match expr_unary.op {
            ast::UnaryOp::Not { .. } => {
                self.asm.push(Inst::Not, span);
            }
            op => {
                return Err(CompileError::UnsupportedUnaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    /// Encode a ref `&<expr>` value.
    fn compile_ref(&mut self, expr: &ast::Expr, _: Span, _: Needs) -> Result<()> {
        // TODO: one day this might be supported in one way or another.
        Err(CompileError::UnsupportedRef { span: expr.span() })
    }

    fn compile_expr_binary(&mut self, expr_binary: &ast::ExprBinary, needs: Needs) -> Result<()> {
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
                    needs,
                )?;
                return Ok(());
            }
            _ => (),
        }

        // NB: need to declare these as anonymous local variables so that they
        // get cleaned up in case there is an early break (return, try, ...).
        self.compile_expr(&*expr_binary.lhs, Needs::Value)?;
        self.scopes.last_mut(span)?.decl_anon(span);

        self.compile_expr(&*expr_binary.rhs, rhs_needs_of(expr_binary.op))?;
        self.scopes.last_mut(span)?.decl_anon(span);

        match expr_binary.op {
            ast::BinOp::Add { .. } => {
                self.asm.push(Inst::Add, span);
            }
            ast::BinOp::Sub { .. } => {
                self.asm.push(Inst::Sub, span);
            }
            ast::BinOp::Div { .. } => {
                self.asm.push(Inst::Div, span);
            }
            ast::BinOp::Mul { .. } => {
                self.asm.push(Inst::Mul, span);
            }
            ast::BinOp::Eq { .. } => {
                self.asm.push(Inst::Eq, span);
            }
            ast::BinOp::Neq { .. } => {
                self.asm.push(Inst::Neq, span);
            }
            ast::BinOp::Lt { .. } => {
                self.asm.push(Inst::Lt, span);
            }
            ast::BinOp::Gt { .. } => {
                self.asm.push(Inst::Gt, span);
            }
            ast::BinOp::Lte { .. } => {
                self.asm.push(Inst::Lte, span);
            }
            ast::BinOp::Gte { .. } => {
                self.asm.push(Inst::Gte, span);
            }
            ast::BinOp::Is { .. } => {
                self.asm.push(Inst::Is, span);
            }
            ast::BinOp::IsNot { .. } => {
                self.asm.push(Inst::IsNot, span);
            }
            ast::BinOp::And { .. } => {
                self.asm.push(Inst::And, span);
            }
            ast::BinOp::Or { .. } => {
                self.asm.push(Inst::Or, span);
            }
            op => {
                return Err(CompileError::UnsupportedBinaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.last_mut(span)?.undecl_anon(2, span)?;
        return Ok(());

        /// Get the need of the right-hand side operator from the type of the
        /// operator.
        fn rhs_needs_of(op: ast::BinOp) -> Needs {
            match op {
                ast::BinOp::Is | ast::BinOp::IsNot => Needs::Type,
                _ => Needs::Value,
            }
        }
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

                self.compile_expr(&*expr, Needs::Value)?;
                self.asm.jump_if(then_label, span);

                Ok(self.scopes.last(span)?.child())
            }
            ast::Condition::ExprLet(expr_let) => {
                let span = expr_let.span();

                let false_label = self.asm.new_label("if_condition_false");

                let mut scope = self.scopes.last(span)?.child();
                self.compile_expr(&*expr_let.expr, Needs::Value)?;

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

    fn compile_expr_if(&mut self, expr_if: &ast::ExprIf, needs: Needs) -> Result<()> {
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
            self.compile_expr_block(&*fallback.block, needs)?;
        } else {
            // NB: if we must produce a value and there is no fallback branch,
            // encode the result of the statement as a unit.
            if needs.value() {
                self.asm.push(Inst::Unit, span);
            }
        }

        self.asm.jump(end_label, span);

        self.asm.label(then_label)?;

        let expected = self.scopes.push(then_scope);
        self.compile_expr_block(&*expr_if.block, needs)?;
        self.clean_last_scope(span, expected, needs)?;

        if !expr_if.expr_else_ifs.is_empty() {
            self.asm.jump(end_label, span);
        }

        let mut it = branches.into_iter().peekable();

        if let Some((branch, label, scope)) = it.next() {
            let span = branch.span();

            self.asm.label(label)?;

            let scopes = self.scopes.push(scope);
            self.compile_expr_block(&*branch.block, needs)?;
            self.clean_last_scope(span, scopes, needs)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;
        Ok(())
    }

    fn compile_expr_match(&mut self, expr_match: &ast::ExprMatch, needs: Needs) -> Result<()> {
        let span = expr_match.span();
        log::trace!("ExprMatch => {:?}", self.source.source(span)?);

        let new_scope = self.scopes.last(span)?.child();
        let expected_scopes = self.scopes.push(new_scope);

        self.compile_expr(&*expr_match.expr, Needs::Value)?;
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
                asm.push(Inst::Copy { offset }, span);
            };

            self.compile_pat(&mut scope, &branch.pat, match_false, &load)?;

            let scope = if let Some((_, condition)) = &branch.condition {
                let span = condition.span();

                let parent_guard = self.scopes.push(scope);
                let scope = self.scopes.last(span)?.child();
                let guard = self.scopes.push(scope);

                self.compile_expr(&*condition, Needs::Value)?;
                self.clean_last_scope(span, guard, Needs::Value)?;
                let scope = self.scopes.pop(span, parent_guard)?;

                self.asm
                    .pop_and_jump_if_not(scope.local_var_count, match_false, span);

                self.asm.jump(branch_label, span);
                scope
            } else {
                scope
            };

            self.asm.jump(branch_label, span);
            self.asm.label(match_false)?;

            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        self.asm.jump(end_label, span);

        let mut it = expr_match.branches.iter().zip(&branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            self.asm.label(*label)?;

            let expected = self.scopes.push(scope.clone());
            self.compile_expr(&*branch.body, needs)?;
            self.clean_last_scope(span, expected, needs)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;

        // pop the implicit scope where we store the anonymous match variable.
        self.clean_last_scope(span, expected_scopes, needs)?;
        Ok(())
    }

    /// Compile an await expression.
    fn compile_expr_await(&mut self, expr_await: &ast::ExprAwait, needs: Needs) -> Result<()> {
        let span = expr_await.span();
        log::trace!("ExprAwait => {:?}", self.source.source(span)?);

        self.compile_expr(&*expr_await.expr, Needs::Value)?;
        self.asm.push(Inst::Await, span);

        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    /// Compile a try expression.
    fn compile_expr_try(&mut self, expr_try: &ast::ExprTry, needs: Needs) -> Result<()> {
        let span = expr_try.span();
        log::trace!("ExprTry => {:?}", self.source.source(span)?);

        let not_error = self.asm.new_label("try_not_error");

        self.compile_expr(&*expr_try.expr, Needs::Value)?;
        self.asm.push(Inst::Dup, span);
        self.asm.push(Inst::IsValue, span);
        self.asm.jump_if(not_error, span);

        // Clean up all locals so far and return from the current function.
        let total_var_count = self.scopes.last(span)?.total_var_count;
        self.locals_clean(total_var_count, span);
        self.asm.push(Inst::Return, span);

        self.asm.label(not_error)?;

        if needs.value() {
            self.asm.push(Inst::Unwrap, span);
        } else {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    /// Compile a select expression.
    fn compile_expr_select(&mut self, expr_select: &ast::ExprSelect, needs: Needs) -> Result<()> {
        let span = expr_select.span();
        log::trace!("ExprSelect => {:?}", self.source.source(span)?);
        let len = expr_select.branches.len();
        self.contexts.push(span);

        let mut branches = Vec::new();

        let end_label = self.asm.new_label("select_end");
        let default_branch = self.asm.new_label("select_default");

        for (branch, _) in &expr_select.branches {
            let label = self.asm.new_label("select_branch");
            branches.push((label, branch));
        }

        for (_, branch) in branches.iter().rev() {
            self.compile_expr(&branch.expr, Needs::Value)?;
        }

        self.asm.push(Inst::Select { len }, span);

        for (branch, (label, _)) in branches.iter().enumerate() {
            self.asm.jump_if_branch(branch, *label, span);
        }

        if expr_select.default_branch.is_some() {
            self.asm.jump(default_branch, span);
        }

        if needs.value() {
            self.asm.push(Inst::Unit, span);
            self.asm.jump(end_label, span);
        }

        for (label, branch) in branches {
            let span = branch.span();
            self.asm.label(label)?;

            let mut scope = self.scopes.last(span)?.child();

            // NB: loop is actually useful.
            #[allow(clippy::never_loop)]
            loop {
                match &branch.pat {
                    ast::Pat::PatPath(path) => {
                        let item = self.convert_path_to_item(&path.path)?;

                        if let Some(local) = item.as_local() {
                            scope.decl_var(local, span);
                            break;
                        }
                    }
                    ast::Pat::PatIgnore(..) => {
                        self.asm.push(Inst::Pop, span);
                        break;
                    }
                    _ => (),
                }

                return Err(CompileError::UnsupportedSelectPattern {
                    span: branch.span(),
                });
            }

            // Set up a new scope with the binding.
            let expected = self.scopes.push(scope);
            self.compile_expr(&*branch.body, needs)?;
            self.clean_last_scope(span, expected, needs)?;
            self.asm.jump(end_label, span);
        }

        if let Some((branch, _)) = &expr_select.default_branch {
            self.asm.label(default_branch)?;
            self.compile_expr(&branch.body, needs)?;
        }

        self.asm.label(end_label)?;

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent context", span))?;

        Ok(())
    }

    /// Encode a vector pattern match.
    fn compile_pat_vec(
        &mut self,
        scope: &mut Scope,
        pat_vec: &ast::PatVec,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = pat_vec.span();
        log::trace!("PatVec => {:?}", self.source.source(span)?);

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed a vector.
        self.asm.push(Inst::Copy { offset }, span);

        self.asm.push(
            Inst::MatchSequence {
                type_check: TypeCheck::Vec,
                len: pat_vec.items.len(),
                exact: pat_vec.open_pattern.is_none(),
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for (index, (pat, _)) in pat_vec.items.iter().enumerate() {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            };

            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode a vector pattern match.
    fn compile_pat_tuple(
        &mut self,
        scope: &mut Scope,
        pat_tuple: &ast::PatTuple,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = pat_tuple.span();
        log::trace!("PatTuple => {:?}", self.source.source(span)?);

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

        let type_check = if let Some(path) = &pat_tuple.path {
            let item = self.convert_path_to_item(path)?;

            let (tuple, type_check) = if let Some(meta) = self.lookup_meta(&item, path.span())? {
                match meta {
                    Meta::MetaTuple { tuple } => {
                        let type_check = TypeCheck::Type(Hash::type_hash(&tuple.item));
                        (tuple, type_check)
                    }
                    Meta::MetaTupleVariant { tuple, .. } => {
                        let type_check = TypeCheck::Variant(Hash::type_hash(&tuple.item));
                        (tuple, type_check)
                    }
                    _ => return Err(CompileError::UnsupportedMetaPattern { meta, span }),
                }
            } else {
                return Err(CompileError::UnsupportedPattern { span });
            };

            let count = pat_tuple.items.len();
            let is_open = pat_tuple.open_pattern.is_some();

            if !(tuple.args == count || count < tuple.args && is_open) {
                return Err(CompileError::UnsupportedArgumentCount {
                    span,
                    meta: Meta::MetaTuple {
                        tuple: tuple.clone(),
                    },
                    expected: tuple.args,
                    actual: count,
                });
            }

            match self.context.type_check_for(&tuple.item) {
                Some(type_check) => type_check,
                None => type_check,
            }
        } else {
            TypeCheck::Tuple
        };

        self.asm.push(Inst::Copy { offset }, span);
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: pat_tuple.items.len(),
                exact: pat_tuple.open_pattern.is_none(),
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for (index, (pat, _)) in pat_tuple.items.iter().enumerate() {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            };

            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode an object pattern match.
    fn compile_pat_object(
        &mut self,
        scope: &mut Scope,
        pat_object: &ast::PatObject,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<()> {
        let span = pat_object.span();
        log::trace!("PatObject => {:?}", self.source.source(span)?);

        // NB: bind the loaded variable (once) to an anonymous var.
        // We reduce the number of copy operations by having specialized
        // operations perform the load from the given offset.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

        let mut string_slots = Vec::new();

        let mut keys_dup = HashMap::new();
        let mut keys = Vec::new();

        for (item, _) in &pat_object.fields {
            let span = item.span();

            let key = item.key.resolve(self.source)?;
            string_slots.push(self.unit.borrow_mut().new_static_string(&*key)?);
            keys.push(key.to_string());

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: pat_object.span(),
                });
            }
        }

        let keys = self.unit.borrow_mut().new_static_object_keys(&keys[..])?;

        let type_check = match &pat_object.ident {
            ast::LitObjectIdent::Named(path) => {
                let span = path.span();
                let item = self.convert_path_to_item(path)?;

                let meta = match self.lookup_meta(&item, span)? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::MissingType { span, item });
                    }
                };

                let (object, type_check) = match &meta {
                    Meta::MetaObject { object } => {
                        let type_check = TypeCheck::Type(Hash::type_hash(&object.item));
                        (object, type_check)
                    }
                    Meta::MetaObjectVariant { object, .. } => {
                        let type_check = TypeCheck::Variant(Hash::type_hash(&object.item));
                        (object, type_check)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedMetaPattern { meta, span });
                    }
                };

                let fields = match &object.fields {
                    Some(fields) => fields,
                    None => {
                        // NB: might want to describe that field composition is unknown because it is an external meta item.
                        return Err(CompileError::UnsupportedMetaPattern { meta, span });
                    }
                };

                for (field, _) in &pat_object.fields {
                    let span = field.key.span();
                    let key = field.key.resolve(self.source)?;

                    if !fields.contains(&*key) {
                        return Err(CompileError::LitObjectNotField {
                            span,
                            field: key.to_string(),
                            item: object.item.clone(),
                        });
                    }
                }

                type_check
            }
            ast::LitObjectIdent::Anonymous(..) => TypeCheck::Object,
        };

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed a vector.
        self.asm.push(Inst::Copy { offset }, span);
        self.asm.push(
            Inst::MatchObject {
                type_check,
                slot: keys,
                exact: pat_object.open_pattern.is_none(),
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for ((item, _), slot) in pat_object.fields.iter().zip(string_slots) {
            let span = item.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::ObjectSlotIndexGetAt { offset, slot }, span);
            };

            if let Some((_, pat)) = &item.binding {
                // load the given vector index and declare it as a local variable.
                self.compile_pat(scope, &*pat, false_label, &load)?;
                continue;
            }

            // NB: only raw identifiers are supported as anonymous bindings
            let ident = match &item.key {
                ast::LitObjectKey::Ident(ident) => ident,
                _ => return Err(CompileError::UnsupportedBinding { span }),
            };

            load(&mut self.asm);
            let name = ident.resolve(self.source)?;
            scope.decl_var(name, span);
        }

        Ok(())
    }

    /// Compile a binding name that matches a known meta type.
    ///
    /// Returns `true` if the binding was used.
    fn compile_pat_meta_binding(
        &mut self,
        scope: &mut Scope,
        span: Span,
        meta: &Meta,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> Result<bool> {
        let (tuple, type_check) = match meta {
            Meta::MetaTuple { tuple } if tuple.args == 0 => {
                (tuple, TypeCheck::Type(Hash::type_hash(&tuple.item)))
            }
            Meta::MetaTupleVariant { tuple, .. } if tuple.args == 0 => {
                (tuple, TypeCheck::Variant(Hash::type_hash(&tuple.item)))
            }
            _ => return Ok(false),
        };

        let type_check = match self.context.type_check_for(&tuple.item) {
            Some(type_check) => type_check,
            None => type_check,
        };

        load(&mut self.asm);
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: tuple.args,
                exact: true,
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);
        Ok(true)
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

        match pat {
            ast::Pat::PatPath(path) => {
                let span = path.span();

                let item = self.convert_path_to_item(&path.path)?;

                if let Some(meta) = self.lookup_meta(&item, span)? {
                    if self.compile_pat_meta_binding(scope, span, &meta, false_label, load)? {
                        return Ok(true);
                    }
                }

                let ident = match item.as_local() {
                    Some(ident) => ident,
                    None => {
                        return Err(CompileError::UnsupportedBinding { span });
                    }
                };

                load(&mut self.asm);
                scope.decl_var(&ident, span);
                return Ok(false);
            }
            ast::Pat::PatIgnore(..) => {
                return Ok(false);
            }
            ast::Pat::PatUnit(unit) => {
                load(&mut self.asm);
                self.asm.push(Inst::IsUnit, unit.span());
            }
            ast::Pat::PatByte(lit_byte) => {
                let byte = lit_byte.resolve(self.source)?;
                load(&mut self.asm);
                self.asm.push(Inst::EqByte { byte }, lit_byte.span());
            }
            ast::Pat::PatChar(lit_char) => {
                let character = lit_char.resolve(self.source)?;
                load(&mut self.asm);
                self.asm
                    .push(Inst::EqCharacter { character }, lit_char.span());
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
                self.asm.push(Inst::EqInteger { integer }, span);
            }
            ast::Pat::PatString(pat_string) => {
                let span = pat_string.span();
                let string = pat_string.resolve(self.source)?;
                let slot = self.unit.borrow_mut().new_static_string(&*string)?;
                load(&mut self.asm);
                self.asm.push(Inst::EqStaticString { slot }, span);
            }
            ast::Pat::PatVec(pat_vec) => {
                self.compile_pat_vec(scope, pat_vec, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatTuple(pat_tuple) => {
                self.compile_pat_tuple(scope, pat_tuple, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatObject(object) => {
                self.compile_pat_object(scope, object, false_label, &load)?;
                return Ok(true);
            }
        }

        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);
        Ok(true)
    }

    /// Clean the last scope.
    fn clean_last_scope(&mut self, span: Span, expected: ScopeGuard, needs: Needs) -> Result<()> {
        let scope = self.scopes.pop(span, expected)?;

        if needs.value() {
            self.locals_clean(scope.local_var_count, span);
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }
}
