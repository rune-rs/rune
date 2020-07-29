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
    /// Parent scope, if available.
    parent: Option<Box<Locals>>,
    locals: HashMap<String, Local>,
    var_count: usize,
    local_count: usize,
}

impl Locals {
    /// Construct a new locals handlers.
    pub fn new() -> Locals {
        Self {
            parent: None,
            locals: HashMap::new(),
            var_count: 0,
            local_count: 0,
        }
    }

    /// Construct a new locals builder with a parent.
    pub fn with_parent(parent: Self) -> Self {
        let var_count = parent.var_count;

        Self {
            parent: Some(Box::new(parent)),
            locals: HashMap::new(),
            var_count,
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

    /// Access the local with the given name.
    pub fn get<'a>(&'a self, name: &str) -> Option<&'a Local> {
        let mut cur = Some(self);

        while let Some(c) = cur {
            if let Some(local) = c.locals.get(name) {
                return Some(local);
            }

            cur = c.parent.as_ref().map(|l| &**l);
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

            let mut instructions = Vec::new();

            let mut encoder = Encoder {
                unit: &mut unit,
                instructions: &mut instructions,
                locals: Locals::new(),
                source,
            };

            encoder.encode_fn_decl(f)?;
            unit.new_function(&[name], count, &instructions)?;
        }

        Ok(unit)
    }
}

struct Encoder<'a> {
    unit: &'a mut st::Unit,
    instructions: &'a mut Vec<st::Inst>,
    locals: Locals,
    source: Source<'a>,
}

impl<'a> Encoder<'a> {
    fn encode_fn_decl(&mut self, fn_decl: ast::FnDecl) -> Result<(), EncodeError> {
        for arg in fn_decl.args.items.into_iter().rev() {
            let name = arg.resolve(self.source)?;
            self.locals.new_local(name, arg.token)?;
        }

        for expr in fn_decl.body.exprs {
            self.encode_block_expr(expr, true)?;
        }

        if let Some(expr) = fn_decl.body.implicit_return {
            self.encode_block_expr(expr, false)?;
            self.instructions.push(st::Inst::Return);
        } else {
            self.instructions.push(st::Inst::ReturnUnit);
        }

        Ok(())
    }

    /// Encode a block.
    fn encode_block(&mut self, block: ast::Block) -> Result<(), EncodeError> {
        for expr in block.exprs {
            self.encode_block_expr(expr, true)?;
        }

        if let Some(expr) = block.implicit_return {
            self.encode_block_expr(expr, false)?;
        } else {
            self.instructions.push(st::Inst::Unit);
        }

        Ok(())
    }

    /// Encode a block expression.
    fn encode_block_expr(&mut self, expr: ast::BlockExpr, pop: bool) -> Result<(), EncodeError> {
        log::trace!("{:?}, pop={:?}", expr, pop);

        match expr {
            ast::BlockExpr::Expr(expr) => {
                self.encode_expr(expr)?;

                if pop {
                    self.instructions.push(st::Inst::Pop);
                }
            }
            ast::BlockExpr::Let(let_) => {
                self.encode_let(let_)?;
            }
        }

        Ok(())
    }

    fn encode_expr(&mut self, expr: ast::Expr) -> Result<(), EncodeError> {
        log::trace!("{:?}", expr);

        match expr {
            ast::Expr::ExprGroup(expr) => {
                self.encode_expr(*expr.expr)?;
            }
            ast::Expr::Ident(ident) => {
                let name = ident.resolve(self.source)?;

                log::trace!("ident={:?}, locals={:?}", name, self.locals);

                let local = self
                    .locals
                    .get(name)
                    .ok_or_else(|| EncodeError::MissingLocal {
                        name: name.to_owned(),
                        span: ident.token.span,
                    })?;

                self.instructions.push(st::Inst::Copy {
                    offset: local.offset,
                });
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
            ast::Expr::NumberLiteral(number) => {
                let number = number.resolve(self.source)?;

                match number {
                    ast::Number::Float(number) => {
                        self.instructions.push(st::Inst::Float { number });
                    }
                    ast::Number::Integer(number) => {
                        self.instructions.push(st::Inst::Integer { number });
                    }
                }
            }
            ast::Expr::ArrayLiteral(array_literal) => {
                let count = array_literal.items.len();

                for expr in array_literal.items.into_iter().rev() {
                    self.encode_expr(expr)?;
                }

                self.instructions.push(st::Inst::Array { count })
            }
            ast::Expr::StringLiteral(string_literal) => {
                let string = string_literal.resolve(self.source)?;
                let slot = self.unit.static_string(&*string)?;
                self.instructions.push(st::Inst::String { slot })
            }
        }

        Ok(())
    }

    fn encode_let(&mut self, let_: ast::Let) -> Result<(), EncodeError> {
        log::trace!("{:?}", let_);

        let name = let_.name.resolve(self.source)?;
        self.encode_expr(let_.expr)?;
        self.locals.new_local(name, let_.name.token)?;
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

        self.encode_expr(*expr_if.condition)?;

        let length = self.instructions.len();

        let mut then_branch = Vec::new();

        Encoder {
            unit: &mut *self.unit,
            instructions: &mut then_branch,
            locals: Locals::with_parent(self.locals.clone()),
            source: self.source,
        }
        .encode_block(*expr_if.then_branch)?;

        if let Some(expr_if_else) = expr_if.expr_if_else {
            let mut else_branch = Vec::new();

            Encoder {
                unit: &mut *self.unit,
                instructions: &mut else_branch,
                locals: Locals::with_parent(self.locals.clone()),
                source: self.source,
            }
            .encode_block(*expr_if_else.else_branch)?;

            // Jump to else branch.
            self.instructions.push(st::Inst::JumpIfNot {
                offset: length + 2 + then_branch.len(),
            });
            // Jump from end of then branch to end of blocks.
            then_branch.push(st::Inst::Jump {
                offset: length + 2 + then_branch.len() + else_branch.len(),
            });

            self.instructions.append(&mut then_branch);
            self.instructions.append(&mut else_branch);
        } else {
            // +1 for the JumpIfNot instruction added
            self.instructions.push(st::Inst::JumpIfNot {
                offset: length + 1 + then_branch.len(),
            });
            self.instructions.append(&mut then_branch);
        }

        Ok(())
    }
}
