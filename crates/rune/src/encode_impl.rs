use crate::ast;
use crate::collections::HashMap;
use crate::error::EncodeError;
use crate::source::Source;
use crate::token::Token;
use crate::traits::Resolve as _;
use crate::ParseAll;

/// Decode the specified path.
fn resolve_path<'a>(path: ast::Path, source: Source<'a>) -> Result<Vec<&'a str>, EncodeError> {
    let mut output = Vec::new();

    output.push(path.first.resolve(source)?);

    for (_, ident) in path.rest {
        output.push(ident.resolve(source)?);
    }

    Ok(output)
}

/// A locally declared variable.
#[derive(Debug, Clone)]
struct Local {
    /// Slot offset from the current stack frame.
    offset: usize,
    /// Name of the variable.
    name: String,
    /// Token assocaited with the variable.
    token: Token,
}

#[derive(Debug, Clone)]
struct Locals {
    locals: HashMap<String, Local>,
    var_count: usize,
    local_count: usize,
}

impl Locals {
    /// Construct a new locals handlers.
    pub fn new() -> Locals {
        Self {
            locals: HashMap::new(),
            var_count: 0,
            local_count: 0,
        }
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub fn new_local(&mut self, name: &str, token: Token) -> Result<(), EncodeError> {
        let local = Local {
            offset: self.var_count,
            name: name.to_owned(),
            token,
        };

        self.var_count += 1;
        self.local_count += 1;

        if let Some(old) = self.locals.insert(name.to_owned(), local) {
            return Err(EncodeError::VariableConflict {
                name: name.to_owned(),
                span: token.span,
                existing_span: old.token.span,
            });
        }

        Ok(())
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub fn decl_var(&mut self, name: &str, token: Token) -> Result<(), usize> {
        if let Some(old) = self.locals.get(name) {
            return Err(old.offset);
        }

        self.locals.insert(
            name.to_owned(),
            Local {
                offset: self.var_count,
                name: name.to_owned(),
                token,
            },
        );

        self.var_count += 1;
        self.local_count += 1;
        Ok(())
    }

    /// Access the local with the given name.
    pub fn get_offset(&self, name: &str) -> Option<usize> {
        if let Some(local) = self.locals.get(name) {
            return Some(local.offset);
        }

        None
    }
}

impl<'a> crate::ParseAll<'a, ast::File> {
    /// Encode the given object into a collection of instructions.
    pub fn encode(self) -> Result<st::Unit, EncodeError> {
        let ParseAll { source, item: file } = self;

        let mut unit = st::Unit::with_default_prelude();

        for import in file.imports {
            let name = resolve_path(import.path, source)?;
            unit.new_import(&name)?;
        }

        for f in file.functions {
            let name = f.name.resolve(source)?;
            let count = f.args.items.len();

            let mut assembler = st::Assembler::new();

            let mut encoder = Encoder {
                unit: &mut unit,
                instructions: &mut assembler,
                parents: Vec::new(),
                locals: Locals::new(),
                source,
                level: 0,
            };

            encoder.encode_fn_decl(f)?;
            unit.new_function(&[name], count, assembler.assembly()?)?;
        }

        Ok(unit)
    }
}

struct Encoder<'a> {
    unit: &'a mut st::Unit,
    instructions: &'a mut st::Assembler,
    parents: Vec<Locals>,
    locals: Locals,
    source: Source<'a>,
    level: usize,
}

impl<'a> Encoder<'a> {
    fn encode_fn_decl(&mut self, fn_decl: ast::FnDecl) -> Result<(), EncodeError> {
        for arg in fn_decl.args.items.into_iter() {
            let name = arg.resolve(self.source)?;
            self.locals.new_local(name, arg.token)?;
        }

        if fn_decl.body.exprs.is_empty() && fn_decl.body.trailing_expr.is_none() {
            self.instructions.push(st::Inst::ReturnUnit);
            return Ok(());
        }

        for (expr, _) in fn_decl.body.exprs {
            let is_empty = expr.is_empty();
            self.encode_expr(expr)?;

            if !is_empty {
                self.instructions.push(st::Inst::Pop);
            }
        }

        if let Some(expr) = fn_decl.body.trailing_expr {
            let is_empty = expr.is_empty();
            self.encode_expr(expr)?;

            if is_empty {
                self.instructions.push(st::Inst::ReturnUnit);
            } else {
                self.instructions.push(st::Inst::Return);
            }
        } else {
            self.instructions.push(st::Inst::ReturnUnit);
        }

        Ok(())
    }

    /// Encode a block.
    fn encode_block(&mut self, block: ast::Block) -> Result<(), EncodeError> {
        log::trace!("{:?}", block);
        let span = block.span();

        self.parents.push(self.locals.clone());

        for (expr, _) in block.exprs {
            let is_empty = expr.is_empty();
            self.encode_expr(expr)?;

            if !is_empty {
                self.instructions.push(st::Inst::Pop);
            }
        }

        if let Some(expr) = block.trailing_expr {
            let is_empty = expr.is_empty();
            self.encode_expr(expr)?;

            if is_empty {
                self.instructions.push(st::Inst::Unit);
            }
        }

        let parent = self
            .parents
            .pop()
            .ok_or_else(|| EncodeError::MissingParentScope { span })?;

        self.locals = parent;
        Ok(())
    }

    /// Encode an expression.
    fn encode_expr(&mut self, expr: ast::Expr) -> Result<(), EncodeError> {
        log::trace!("{:?}", expr);

        match expr {
            ast::Expr::While(while_) => {
                self.encode_while(while_)?;
            }
            ast::Expr::Let(let_) => {
                self.encode_let(let_)?;
            }
            ast::Expr::Update(let_) => {
                self.encode_update(let_)?;
            }
            ast::Expr::IndexSet(index_set) => {
                self.encode_index_set(index_set)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.encode_expr(*expr.expr)?;
            }
            ast::Expr::Ident(ident) => {
                self.encode_local_copy(ident)?;
            }
            ast::Expr::CallFn(call_fn) => {
                self.encode_call_fn(call_fn)?;
            }
            ast::Expr::CallInstanceFn(call_instance_fn) => {
                self.encode_call_instance_fn(call_instance_fn)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.encode_expr_binary(expr_binary)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.encode_expr_if(expr_if)?;
            }
            ast::Expr::UnitLiteral(unit) => {
                self.encode_unit_literal(unit)?;
            }
            ast::Expr::BoolLiteral(b) => {
                self.encode_bool_literal(b)?;
            }
            ast::Expr::NumberLiteral(number) => {
                self.encode_number_literal(number)?;
            }
            ast::Expr::ArrayLiteral(array_literal) => {
                let count = array_literal.items.len();

                for expr in array_literal.items.into_iter().rev() {
                    self.encode_expr(expr)?;
                }

                self.instructions.push(st::Inst::Array { count })
            }
            ast::Expr::ObjectLiteral(object_literal) => {
                let count = object_literal.items.len();

                for (key, _, value) in object_literal.items.into_iter().rev() {
                    self.encode_expr(value)?;
                    self.encode_string_literal(key)?;
                }

                self.instructions.push(st::Inst::Object { count })
            }
            ast::Expr::StringLiteral(string) => {
                self.encode_string_literal(string)?;
            }
            ast::Expr::IndexGet(index_get) => {
                self.encode_index_get(index_get)?;
            }
        }

        Ok(())
    }

    fn encode_string_literal(&mut self, string: ast::StringLiteral) -> Result<(), EncodeError> {
        let string = string.resolve(self.source)?;
        let slot = self.unit.static_string(&*string)?;
        self.instructions.push(st::Inst::String { slot });
        Ok(())
    }

    fn encode_unit_literal(&mut self, _: ast::UnitLiteral) -> Result<(), EncodeError> {
        self.instructions.push(st::Inst::Unit);
        Ok(())
    }

    fn encode_bool_literal(&mut self, b: ast::BoolLiteral) -> Result<(), EncodeError> {
        self.instructions.push(st::Inst::Bool { value: b.value });
        Ok(())
    }

    fn encode_number_literal(&mut self, number: ast::NumberLiteral) -> Result<(), EncodeError> {
        let number = number.resolve(self.source)?;

        match number {
            ast::Number::Float(number) => {
                self.instructions.push(st::Inst::Float { number });
            }
            ast::Number::Integer(number) => {
                self.instructions.push(st::Inst::Integer { number });
            }
        }

        Ok(())
    }

    fn encode_while(&mut self, while_: ast::While) -> Result<(), EncodeError> {
        log::trace!("{:?}", while_);
        let level = self.level;
        self.level += 1;

        let is_empty = while_.body.is_empty();
        let start_label = format!("while/condition/{}", level);
        let end_label = format!("while/end/{}", level);

        self.instructions.label(&start_label)?;
        self.encode_expr(*while_.condition)?;
        self.instructions.jump_if_not(&end_label);
        self.encode_block(*while_.body)?;

        if is_empty {
            self.instructions.push(st::Inst::Unit);
        }

        self.instructions.jump(&start_label);
        self.instructions.label(&end_label)?;
        Ok(())
    }

    fn encode_let(&mut self, let_: ast::Let) -> Result<(), EncodeError> {
        log::trace!("{:?}", let_);

        let name = let_.name.resolve(self.source)?;
        self.encode_expr(*let_.expr)?;

        if let Err(offset) = self.locals.decl_var(name, let_.name.token) {
            // We are overloading an existing variable, so just replace it.
            self.instructions.push(st::Inst::Replace { offset });
        }

        Ok(())
    }

    fn encode_update(&mut self, update: ast::Update) -> Result<(), EncodeError> {
        log::trace!("{:?}", update);

        let token = update.name.token;
        let name = update.name.resolve(self.source)?;
        self.encode_expr(*update.expr)?;

        let offset = self
            .locals
            .get_offset(name)
            .ok_or_else(|| EncodeError::MissingLocal {
                name: name.to_owned(),
                span: token.span,
            })?;

        self.instructions.push(st::Inst::Replace { offset });
        Ok(())
    }

    fn encode_index_get(&mut self, index_get: ast::IndexGet) -> Result<(), EncodeError> {
        self.encode_expr(*index_get.index)?;
        self.encode_local_copy(index_get.target)?;
        self.instructions.push(st::Inst::IndexGet);
        Ok(())
    }

    fn encode_index_set(&mut self, index_set: ast::IndexSet) -> Result<(), EncodeError> {
        self.encode_expr(*index_set.value)?;
        self.encode_expr(*index_set.index)?;
        self.encode_local_copy(index_set.target)?;
        self.instructions.push(st::Inst::IndexSet);
        Ok(())
    }

    fn encode_local_copy(&mut self, ident: ast::Ident) -> Result<(), EncodeError> {
        let target = ident.resolve(self.source)?;

        let offset = self
            .locals
            .get_offset(target)
            .ok_or_else(|| EncodeError::MissingLocal {
                name: target.to_owned(),
                span: ident.token.span,
            })?;

        self.instructions.push(st::Inst::Copy { offset });
        Ok(())
    }

    /// Decode a path into a call destination based on its hashes.
    fn decode_call_dest(&self, path: ast::Path) -> Result<st::Hash, EncodeError> {
        let local = path.first.resolve(self.source)?;

        let imported = match self.unit.lookup_import_by_name(local).cloned() {
            Some(path) => path,
            None => st::ItemPath::of(&[local]),
        };

        let mut rest = Vec::new();

        for (_, part) in path.rest {
            rest.push(part.resolve(self.source)?);
        }

        let it = imported
            .into_iter()
            .map(String::as_str)
            .chain(rest.into_iter());

        Ok(st::Hash::function(it))
    }

    fn encode_call_fn(&mut self, call_fn: ast::CallFn) -> Result<(), EncodeError> {
        log::trace!("{:?}", call_fn);

        let args = call_fn.args.items.len();

        for expr in call_fn.args.items.into_iter().rev() {
            self.encode_expr(expr)?;
        }

        let hash = self.decode_call_dest(call_fn.name)?;
        self.instructions.push(st::Inst::Call { hash, args });
        Ok(())
    }

    fn encode_call_instance_fn(
        &mut self,
        call_instance_fn: ast::CallInstanceFn,
    ) -> Result<(), EncodeError> {
        log::trace!("{:?}", call_instance_fn);

        let args = call_instance_fn.args.items.len();

        for expr in call_instance_fn.args.items.into_iter().rev() {
            self.encode_expr(expr)?;
        }

        self.encode_expr(*call_instance_fn.instance)?;

        let name = call_instance_fn.name.resolve(self.source)?;
        let hash = st::Hash::of(name);
        self.instructions
            .push(st::Inst::CallInstance { hash, args });
        Ok(())
    }

    fn encode_expr_binary(&mut self, expr_binary: ast::ExprBinary) -> Result<(), EncodeError> {
        log::trace!("{:?}", expr_binary);

        self.encode_expr(*expr_binary.lhs)?;
        self.encode_expr(*expr_binary.rhs)?;

        match expr_binary.op {
            ast::BinOp::Add { .. } => {
                self.instructions.push(st::Inst::Add);
            }
            ast::BinOp::Sub { .. } => {
                self.instructions.push(st::Inst::Sub);
            }
            ast::BinOp::Div { .. } => {
                self.instructions.push(st::Inst::Div);
            }
            ast::BinOp::Mul { .. } => {
                self.instructions.push(st::Inst::Mul);
            }
            ast::BinOp::Eq { .. } => {
                self.instructions.push(st::Inst::Eq);
            }
            ast::BinOp::Neq { .. } => {
                self.instructions.push(st::Inst::Neq);
            }
            ast::BinOp::Lt { .. } => {
                self.instructions.push(st::Inst::Lt);
            }
            ast::BinOp::Gt { .. } => {
                self.instructions.push(st::Inst::Gt);
            }
            ast::BinOp::Lte { .. } => {
                self.instructions.push(st::Inst::Lte);
            }
            ast::BinOp::Gte { .. } => {
                self.instructions.push(st::Inst::Gte);
            }
        }

        Ok(())
    }

    fn encode_expr_if(&mut self, expr_if: ast::ExprIf) -> Result<(), EncodeError> {
        log::trace!("{:?}", expr_if);
        let level = self.level;
        self.level += 1;

        let is_unit = expr_if.expr_else.is_none();

        self.encode_expr(*expr_if.condition)?;
        self.instructions.jump_if(format!("if/branch/{}", level));

        for (i, branch) in expr_if.expr_else_ifs.clone().into_iter().enumerate() {
            self.encode_expr(*branch.condition)?;
            self.instructions
                .jump_if(format!("if/branch/{}/{}", level, i));
        }

        // use fallback as fall through.
        if let Some(fallback) = expr_if.expr_else {
            let is_empty = fallback.block.is_empty();
            self.encode_block(*fallback.block)?;

            if is_empty {
                self.instructions.push(st::Inst::Unit);
            } else if is_unit {
                self.instructions.push(st::Inst::Pop);
                self.instructions.push(st::Inst::Unit);
            }
        } else {
            self.instructions.push(st::Inst::Unit);
        }

        self.instructions.jump(format!("if/end/{}", level));

        self.instructions.label(format!("if/branch/{}", level))?;

        let is_empty = expr_if.block.is_empty();
        self.encode_block(*expr_if.block)?;

        if is_empty {
            self.instructions.push(st::Inst::Unit);
        } else if is_unit {
            self.instructions.push(st::Inst::Pop);
            self.instructions.push(st::Inst::Unit);
        }

        if !expr_if.expr_else_ifs.is_empty() {
            self.instructions.jump(format!("if/end/{}", level));
        }

        let mut it = expr_if.expr_else_ifs.into_iter().enumerate().peekable();

        if let Some((i, branch)) = it.next() {
            let is_empty = branch.block.is_empty();
            self.instructions
                .label(format!("if/branch/{}/{}", level, i))?;
            self.encode_block(*branch.block)?;

            if is_empty {
                self.instructions.push(st::Inst::Unit);
            } else if is_unit {
                self.instructions.push(st::Inst::Pop);
                self.instructions.push(st::Inst::Unit);
            }

            if it.peek().is_some() {
                self.instructions.jump(format!("if/end/{}", level));
            }
        }

        self.instructions.label(format!("if/end/{}", level))?;
        Ok(())
    }
}
