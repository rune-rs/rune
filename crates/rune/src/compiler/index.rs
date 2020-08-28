use crate::ast;
use crate::compiler::query::Query;
use crate::compiler::Items;
use crate::error::CompileError;
use crate::traits::Resolve as _;

pub(super) struct FunctionIndexer<'a, 'source> {
    pub(super) items: Items,
    pub(super) query: &'a mut Query<'source>,
}

pub(super) trait Index<T> {
    /// Walk the current type with the given item.
    fn index(&mut self, item: &T) -> Result<(), CompileError>;
}

impl Index<ast::DeclFn> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::DeclFn) -> Result<(), CompileError> {
        self.index(&item.body)?;
        Ok(())
    }
}

impl Index<ast::ExprBlock> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprBlock) -> Result<(), CompileError> {
        let span = item.span();
        let guard = self.items.push_block();

        for (expr, _) in &item.exprs {
            self.index(expr)?;
        }

        if let Some(expr) = &item.trailing_expr {
            self.index(&**expr)?;
        }

        self.items.pop(guard, span)?;
        Ok(())
    }
}

impl Index<ast::ExprLet> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprLet) -> Result<(), CompileError> {
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::Expr> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::Expr) -> Result<(), CompileError> {
        match item {
            ast::Expr::ExprBlock(block) => {
                self.index(block)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.index(&*expr.expr)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.index(expr_if)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.index(expr_binary)?;
            }
            ast::Expr::ExprMatch(expr_if) => {
                self.index(expr_if)?;
            }
            ast::Expr::Decl(decl) => {
                self.index(decl)?;
            }
            _ => (),
        }

        Ok(())
    }
}

impl Index<ast::ExprIf> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprIf) -> Result<(), CompileError> {
        self.index(&item.condition)?;
        self.index(&*item.block)?;

        for expr_else_if in &item.expr_else_ifs {
            self.index(&expr_else_if.condition)?;
            self.index(&*expr_else_if.block)?;
        }

        if let Some(expr_else) = &item.expr_else {
            self.index(&*expr_else.block)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprBinary> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprBinary) -> Result<(), CompileError> {
        self.index(&*item.lhs)?;
        self.index(&*item.rhs)?;
        Ok(())
    }
}

impl Index<ast::ExprFor> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprFor) -> Result<(), CompileError> {
        self.index(&*item.iter)?;
        self.index(&*item.body)?;
        Ok(())
    }
}

impl Index<ast::ExprMatch> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprMatch) -> Result<(), CompileError> {
        self.index(&*item.expr)?;

        for (branch, _) in &item.branches {
            if let Some((_, condition)) = &branch.condition {
                self.index(&**condition)?;
            }

            self.index(&*branch.body)?;
        }

        Ok(())
    }
}

impl Index<ast::Condition> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::Condition) -> Result<(), CompileError> {
        match item {
            ast::Condition::Expr(expr) => {
                self.index(&**expr)?;
            }
            ast::Condition::ExprLet(expr_let) => {
                self.index(&**expr_let)?;
            }
        }

        Ok(())
    }
}

impl Index<ast::Decl> for FunctionIndexer<'_, '_> {
    fn index(&mut self, item: &ast::Decl) -> Result<(), CompileError> {
        match item {
            ast::Decl::DeclUse(import) => {
                let name = import.path.resolve(self.query.source)?;
                let item = self.items.item();
                self.query.unit.borrow_mut().new_import(item, &name)?;
            }
            ast::Decl::DeclEnum(decl_enum) => {
                let span = decl_enum.span();
                let name = decl_enum.name.resolve(self.query.source)?;
                let guard = self.items.push_name(name);
                let enum_item = self.items.item();
                self.query.new_enum(enum_item.clone());

                for (variant, body, _) in &decl_enum.variants {
                    let span = variant.span();
                    let variant = variant.resolve(self.query.source)?;
                    let guard = self.items.push_name(variant);

                    self.query
                        .new_variant(self.items.item(), enum_item.clone(), body.clone());

                    self.items.pop(guard, span)?;
                }

                self.items.pop(guard, span)?;
            }
            ast::Decl::DeclStruct(decl_struct) => {
                let span = decl_struct.span();
                let name = decl_struct.ident.resolve(self.query.source)?;
                let guard = self.items.push_name(name);
                self.query
                    .new_struct(self.items.item(), decl_struct.clone());
                self.items.pop(guard, span)?;
            }
            ast::Decl::DeclFn(decl_fn) => {
                let span = decl_fn.span();

                let name = decl_fn.name.resolve(self.query.source)?;
                let guard = self.items.push_name(name);

                let item = self.items.item();
                self.query.new_function(item, decl_fn.clone());

                self.items.pop(guard, span)?;
            }
        }

        Ok(())
    }
}
