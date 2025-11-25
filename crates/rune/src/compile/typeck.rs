//! Type checking pass for gradual typing (Phase 2).
//!
//! This module implements type validation for explicitly annotated code
//! in accordance with gradual typing semantics:
//!
//! - Functions with return type annotations are checked against their body
//! - Untyped code is treated as having type `Any` and bypasses checking
//! - Type mismatches produce warnings by default, errors in strict mode
//! - Type inference for expressions (literals, binary ops, blocks, etc.)
//!
//! Phase 2B adds local type inference:
//! - Type variables for inference
//! - Unification algorithm
//! - Variable binding tracking
//! - Expression type inference
//!
//! See GRADUAL_TYPING_PLAN.md for the full implementation roadmap.

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, String, Vec};
use crate::ast::{self, NumberSource, Spanned};
use crate::compile::{self, Location, Options};
use crate::diagnostics::WarningDiagnosticKind;
use crate::indexing::{FunctionAst, Indexed};
use crate::query::Query;
use crate::{SourceId, Sources};

// ============================================================================
// Type Variables for Inference
// ============================================================================

/// A type variable used during type inference.
///
/// Type variables are placeholders that get unified with concrete types
/// during the inference process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TypeVar(usize);

/// Resolved type information for gradual type checking.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    /// A named type (e.g., `i64`, `String`, `foo::Bar`)
    Named(String),
    /// A tuple of types.
    Tuple(Vec<ResolvedType>),
    /// The never type `!`
    Never,
    /// Dynamic/untyped - compatible with everything (gradual typing)
    Any,
    /// A type variable (used during inference)
    Variable(TypeVar),
}

impl TryClone for ResolvedType {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(match self {
            ResolvedType::Named(s) => ResolvedType::Named(s.try_clone()?),
            ResolvedType::Tuple(types) => ResolvedType::Tuple(types.try_clone()?),
            ResolvedType::Never => ResolvedType::Never,
            ResolvedType::Any => ResolvedType::Any,
            ResolvedType::Variable(v) => ResolvedType::Variable(*v),
        })
    }
}

impl ResolvedType {
    /// Convert AST type to resolved type.
    fn from_ast_type(ty: &ast::Type, sources: &Sources, source_id: SourceId) -> compile::Result<Self> {
        match ty {
            ast::Type::Path(path) => {
                // Extract the type name from the path
                let name = path_to_type_string(path, sources, source_id)?;
                Ok(ResolvedType::Named(name))
            }
            ast::Type::Bang(_) => Ok(ResolvedType::Never),
            ast::Type::Tuple(tuple) => {
                let mut types = Vec::new();
                for (inner_ty, _) in tuple.iter() {
                    types.try_push(Self::from_ast_type(inner_ty, sources, source_id)?)?;
                }
                Ok(ResolvedType::Tuple(types))
            }
        }
    }

    /// Get the type of a literal.
    fn from_literal(lit: &ast::Lit) -> compile::Result<Self> {
        Ok(match lit {
            ast::Lit::Bool(_) => ResolvedType::Named(String::try_from("bool")?),
            ast::Lit::Byte(_) => ResolvedType::Named(String::try_from("u8")?),
            ast::Lit::Str(_) => ResolvedType::Named(String::try_from("String")?),
            ast::Lit::ByteStr(_) => ResolvedType::Named(String::try_from("Bytes")?),
            ast::Lit::Char(_) => ResolvedType::Named(String::try_from("char")?),
            ast::Lit::Number(num) => {
                // Check if it's a float or integer
                let is_float = match &num.source {
                    NumberSource::Text(text) => text.is_fractional,
                    NumberSource::Synthetic(_) => false,
                };
                if is_float {
                    ResolvedType::Named(String::try_from("f64")?)
                } else {
                    ResolvedType::Named(String::try_from("i64")?)
                }
            }
        })
    }

    /// Convert to display string.
    fn to_display_string(&self) -> compile::Result<String> {
        Ok(match self {
            ResolvedType::Named(name) => name.try_clone()?,
            ResolvedType::Tuple(types) => {
                let mut result = String::new();
                result.try_push('(')?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        result.try_push_str(", ")?;
                    }
                    result.try_push_str(&ty.to_display_string()?)?;
                }
                result.try_push(')')?;
                result
            }
            ResolvedType::Never => String::try_from("!")?,
            ResolvedType::Any => String::try_from("Any")?,
            ResolvedType::Variable(v) => {
                let mut result = String::new();
                result.try_push_str("?T")?;
                use crate::alloc::fmt::TryWrite;
                write!(result, "{}", v.0)?;
                result
            }
        })
    }

    /// Check if two types are compatible under gradual typing semantics.
    ///
    /// Returns `true` if the types are compatible (no warning needed).
    fn is_compatible_with(&self, other: &Self) -> bool {
        // Any is compatible with everything
        if matches!(self, ResolvedType::Any) || matches!(other, ResolvedType::Any) {
            return true;
        }

        // Type variables are compatible with everything (they'll be resolved later)
        if matches!(self, ResolvedType::Variable(_)) || matches!(other, ResolvedType::Variable(_))
        {
            return true;
        }

        match (self, other) {
            (ResolvedType::Named(a), ResolvedType::Named(b)) => a == b,
            (ResolvedType::Tuple(a), ResolvedType::Tuple(b)) if a.len() == b.len() => {
                a.iter().zip(b.iter()).all(|(a, b)| a.is_compatible_with(b))
            }
            (ResolvedType::Never, _) => true, // Never is subtype of everything
            _ => false,
        }
    }
}

// ============================================================================
// Inference Context
// ============================================================================

/// Context for type inference within a function.
///
/// Tracks type variables, substitutions, and variable bindings.
pub(crate) struct InferenceContext {
    /// Counter for generating fresh type variables
    next_var: usize,
    /// Substitution map: TypeVar -> ResolvedType
    substitutions: HashMap<TypeVar, ResolvedType>,
    /// Variable scope stack for tracking variable types by name
    scopes: Vec<HashMap<String, ResolvedType>>,
}

impl InferenceContext {
    /// Create a new inference context.
    pub fn new() -> alloc::Result<Self> {
        let mut scopes = Vec::new();
        scopes.try_push(HashMap::new())?;
        Ok(Self {
            next_var: 0,
            substitutions: HashMap::new(),
            scopes,
        })
    }

    /// Create a fresh type variable.
    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Push a new variable scope.
    pub fn push_scope(&mut self) -> alloc::Result<()> {
        self.scopes.try_push(HashMap::new())
    }

    /// Pop the current variable scope.
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Bind a variable name to a type in the current scope.
    pub fn bind_var(&mut self, name: String, ty: ResolvedType) -> alloc::Result<()> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.try_insert(name, ty)?;
        }
        Ok(())
    }

    /// Look up a variable's type by name.
    pub fn lookup_var(&self, name: &str) -> compile::Result<ResolvedType> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.try_clone()?);
            }
        }
        // Unknown variable - return Any for gradual typing
        Ok(ResolvedType::Any)
    }

    /// Apply substitutions to resolve a type.
    ///
    /// Recursively replaces type variables with their substituted values.
    pub fn apply(&self, ty: &ResolvedType) -> compile::Result<ResolvedType> {
        Ok(match ty {
            ResolvedType::Variable(v) => {
                if let Some(resolved) = self.substitutions.get(v) {
                    // Recursively apply in case the substitution contains more variables
                    self.apply(resolved)?
                } else {
                    // Unresolved type variable - default to Any for gradual typing
                    ResolvedType::Any
                }
            }
            ResolvedType::Tuple(types) => {
                let mut resolved = Vec::new();
                for t in types {
                    resolved.try_push(self.apply(t)?)?;
                }
                ResolvedType::Tuple(resolved)
            }
            other => other.try_clone()?,
        })
    }

    /// Unify two types, updating the substitution map.
    ///
    /// Returns Ok(()) if unification succeeds, potentially adding new substitutions.
    pub fn unify(&mut self, t1: &ResolvedType, t2: &ResolvedType) -> compile::Result<()> {
        let t1 = self.apply(t1)?;
        let t2 = self.apply(t2)?;

        match (&t1, &t2) {
            // Type variable unification - bind to other type
            (ResolvedType::Variable(v), other) | (other, ResolvedType::Variable(v)) => {
                if !self.occurs_check(*v, other) {
                    self.substitutions.try_insert(*v, other.try_clone()?)?;
                }
                Ok(())
            }
            // Any unifies with everything (gradual typing semantics)
            (ResolvedType::Any, _) | (_, ResolvedType::Any) => Ok(()),
            // Same named types unify
            (ResolvedType::Named(a), ResolvedType::Named(b)) if a == b => Ok(()),
            // Tuple types unify if same arity and elements unify
            (ResolvedType::Tuple(a), ResolvedType::Tuple(b)) if a.len() == b.len() => {
                for (a, b) in a.iter().zip(b.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            }
            // Never is bottom type - unifies with anything
            (ResolvedType::Never, _) | (_, ResolvedType::Never) => Ok(()),
            // Types don't unify - but don't error, let type checking catch it
            _ => Ok(()),
        }
    }

    /// Occurs check to prevent infinite types.
    ///
    /// Returns true if the type variable occurs in the type.
    fn occurs_check(&self, var: TypeVar, ty: &ResolvedType) -> bool {
        match ty {
            ResolvedType::Variable(v) => var == *v,
            ResolvedType::Tuple(types) => types.iter().any(|t| self.occurs_check(var, t)),
            _ => false,
        }
    }
}

/// Convert an AST path to a type string.
fn path_to_type_string(path: &ast::Path, sources: &Sources, source_id: SourceId) -> compile::Result<String> {
    let mut result = String::new();

    if path.global.is_some() {
        result.try_push_str("::")?;
    }

    // Add the first segment
    segment_to_string(&path.first, &mut result, sources, source_id)?;

    // Add remaining segments
    for (_, segment) in &path.rest {
        result.try_push_str("::")?;
        segment_to_string(segment, &mut result, sources, source_id)?;
    }

    Ok(result)
}

/// Convert a path segment to a string.
fn segment_to_string(segment: &ast::PathSegment, result: &mut String, sources: &Sources, source_id: SourceId) -> compile::Result<()> {
    match segment {
        ast::PathSegment::Ident(ident) => {
            // Use the sources to resolve the identifier
            if let Some(name) = sources.source(source_id, ident.span) {
                result.try_push_str(name)?;
            } else {
                result.try_push_str("_unknown")?;
            }
        }
        ast::PathSegment::SelfType(_) => {
            result.try_push_str("Self")?;
        }
        ast::PathSegment::SelfValue(_) => {
            result.try_push_str("self")?;
        }
        ast::PathSegment::Crate(_) => {
            result.try_push_str("crate")?;
        }
        ast::PathSegment::Super(_) => {
            result.try_push_str("super")?;
        }
        ast::PathSegment::Generics(_) => {
            // Skip generics for now - they're complex to resolve without context
            result.try_push_str("<...>")?;
        }
    }
    Ok(())
}

/// Perform type checking on all indexed functions.
pub(crate) fn check_types(q: &mut Query<'_, '_>, options: &Options) -> compile::Result<()> {
    // Collect all functions to check
    let mut functions_to_check = Vec::new();

    for entry in q.inner.indexed_entries() {
        if let Indexed::Function(f) = &entry.indexed {
            if let Ok(ast) = f.ast.try_clone() {
                functions_to_check.try_push((entry.item_meta.location, ast))?;
            }
        }
    }

    // Check each function
    for (location, ast) in functions_to_check {
        check_function(q, location, &ast, options)?;
    }

    Ok(())
}

/// Check a single function for type mismatches.
fn check_function(
    q: &mut Query<'_, '_>,
    location: Location,
    ast: &FunctionAst,
    options: &Options,
) -> compile::Result<()> {
    // Only check functions with full AST (ItemFn)
    let item_fn = match ast {
        FunctionAst::Item(item, _) => item,
        _ => return Ok(()), // Skip non-Item functions for now
    };

    // Check if there's a return type annotation
    let Some((_, return_type)) = &item_fn.output else {
        return Ok(()); // No return type annotation, nothing to check
    };

    let expected_type = ResolvedType::from_ast_type(return_type, q.sources, location.source_id)?;

    // Create an inference context for type inference within this function
    let mut ctx = InferenceContext::new()?;

    // Register parameter types if annotated
    for (arg, _) in item_fn.args.iter() {
        match arg {
            #[cfg(feature = "gradual-typing")]
            ast::FnArg::Typed(typed) => {
                // Get parameter name from pattern
                if let Some(name) = extract_pat_name(&typed.pat, q.sources, location.source_id) {
                    let param_type =
                        ResolvedType::from_ast_type(&typed.ty, q.sources, location.source_id)?;
                    ctx.bind_var(name, param_type)?;
                }
            }
            ast::FnArg::Pat(pat) => {
                // Untyped parameter - bind as Any
                if let Some(name) = extract_pat_name(pat, q.sources, location.source_id) {
                    ctx.bind_var(name, ResolvedType::Any)?;
                }
            }
            ast::FnArg::SelfValue(_) => {
                // Self parameter - use Any for now
                ctx.bind_var(String::try_from("self")?, ResolvedType::Any)?;
            }
        }
    }

    // Infer the type of the function body using the context
    let inferred_type =
        infer_block_type_with_ctx(&mut ctx, q.sources, location.source_id, &item_fn.body)?;
    let actual_type = ctx.apply(&inferred_type)?;

    // Check if the inferred type matches the expected return type
    if !actual_type.is_compatible_with(&expected_type) {
        emit_type_mismatch(
            q,
            location.source_id,
            item_fn.body.span(),
            &expected_type,
            &actual_type,
            options,
        )?;
    }

    // Also check for explicit return statements
    check_block_return_type(q, location.source_id, &item_fn.body, &expected_type, options)?;

    Ok(())
}

/// Extract a variable name from a pattern.
fn extract_pat_name(pat: &ast::Pat, sources: &Sources, source_id: SourceId) -> Option<String> {
    match pat {
        ast::Pat::Path(path) => get_path_ident(&path.path, sources, source_id),
        ast::Pat::Binding(binding) => sources
            .source(source_id, binding.key.span())
            .and_then(|s| String::try_from(s).ok()),
        _ => None,
    }
}

/// Check that explicit return statements in the block match the expected type.
///
/// Note: The implicit return (last expression) is checked by `infer_block_type_with_ctx`,
/// so we only check explicit `return` statements here to avoid duplicate warnings.
fn check_block_return_type(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    block: &ast::Block,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    for stmt in &block.statements {
        match stmt {
            ast::Stmt::Expr(expr) => {
                // Check for explicit returns within the expression (but not the expression itself)
                check_expr_for_returns(q, source_id, expr, expected, options)?;
            }
            ast::Stmt::Semi(semi) => {
                // Check for explicit returns within the expression
                check_expr_for_returns(q, source_id, &semi.expr, expected, options)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check if an expression's type matches the expected type.
fn check_expr_type(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    expr: &ast::Expr,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    let actual = infer_expr_type(expr)?;

    if !actual.is_compatible_with(expected) {
        emit_type_mismatch(q, source_id, expr.span(), expected, &actual, options)?;
    }

    Ok(())
}

/// Check an expression tree for return statements.
fn check_expr_for_returns(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    expr: &ast::Expr,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    match expr {
        ast::Expr::Return(ret) => {
            if let Some(ret_expr) = &ret.expr {
                check_expr_type(q, source_id, ret_expr, expected, options)?;
            }
        }
        ast::Expr::Block(block) => {
            check_block_return_type(q, source_id, &block.block, expected, options)?;
        }
        ast::Expr::If(if_expr) => {
            check_block_return_type(q, source_id, &if_expr.block, expected, options)?;
            for branch in &if_expr.expr_else_ifs {
                check_block_return_type(q, source_id, &branch.block, expected, options)?;
            }
            if let Some(else_branch) = &if_expr.expr_else {
                check_block_return_type(q, source_id, &else_branch.block, expected, options)?;
            }
        }
        ast::Expr::Match(match_expr) => {
            for (branch, _) in &match_expr.branches {
                check_expr_for_returns(q, source_id, &branch.body, expected, options)?;
            }
        }
        // For other expression types, we just look for nested returns
        _ => {}
    }

    Ok(())
}

/// Infer the type of an expression using the inference context.
fn infer_expr_type_with_ctx(
    ctx: &mut InferenceContext,
    sources: &Sources,
    source_id: SourceId,
    expr: &ast::Expr,
) -> compile::Result<ResolvedType> {
    Ok(match expr {
        // Literals - concrete known types
        ast::Expr::Lit(lit) => ResolvedType::from_literal(&lit.lit)?,

        // Tuples - recurse into each element
        ast::Expr::Tuple(tuple) => {
            let mut types = Vec::new();
            for (e, _) in tuple.items.iter() {
                types.try_push(infer_expr_type_with_ctx(ctx, sources, source_id, e)?)?;
            }
            ResolvedType::Tuple(types)
        }

        // Binary operations - infer from operand types
        ast::Expr::Binary(binary) => {
            let lhs = infer_expr_type_with_ctx(ctx, sources, source_id, &binary.lhs)?;
            let rhs = infer_expr_type_with_ctx(ctx, sources, source_id, &binary.rhs)?;

            match &binary.op {
                // Arithmetic operations - result type same as operands
                ast::BinOp::Add(_)
                | ast::BinOp::Sub(_)
                | ast::BinOp::Mul(_)
                | ast::BinOp::Div(_)
                | ast::BinOp::Rem(_) => {
                    ctx.unify(&lhs, &rhs)?;
                    ctx.apply(&lhs)?
                }
                // Comparison operations - return bool
                ast::BinOp::Eq(_)
                | ast::BinOp::Neq(_)
                | ast::BinOp::Lt(_)
                | ast::BinOp::Gt(_)
                | ast::BinOp::Lte(_)
                | ast::BinOp::Gte(_) => ResolvedType::Named(String::try_from("bool")?),
                // Logical operations - return bool
                ast::BinOp::And(_) | ast::BinOp::Or(_) => {
                    ResolvedType::Named(String::try_from("bool")?)
                }
                // Other operations - return Any
                _ => ResolvedType::Any,
            }
        }

        // Unary operations
        ast::Expr::Unary(unary) => {
            let operand = infer_expr_type_with_ctx(ctx, sources, source_id, &unary.expr)?;
            match &unary.op {
                ast::UnOp::Not(_) => ResolvedType::Named(String::try_from("bool")?),
                ast::UnOp::Neg(_) => operand,
                _ => ResolvedType::Any,
            }
        }

        // Block expressions - type is the last expression's type
        ast::Expr::Block(block) => {
            infer_block_type_with_ctx(ctx, sources, source_id, &block.block)?
        }

        // If expressions - unify all branches
        ast::Expr::If(if_expr) => {
            let then_type = infer_block_type_with_ctx(ctx, sources, source_id, &if_expr.block)?;

            // Check else-if branches
            for branch in &if_expr.expr_else_ifs {
                let branch_type =
                    infer_block_type_with_ctx(ctx, sources, source_id, &branch.block)?;
                ctx.unify(&then_type, &branch_type)?;
            }

            // Check else branch
            if let Some(else_branch) = &if_expr.expr_else {
                let else_type =
                    infer_block_type_with_ctx(ctx, sources, source_id, &else_branch.block)?;
                ctx.unify(&then_type, &else_type)?;
            }

            ctx.apply(&then_type)?
        }

        // Match expressions - unify all branch types
        ast::Expr::Match(match_expr) => {
            let mut result_type = ResolvedType::Variable(ctx.fresh_var());

            for (branch, _) in &match_expr.branches {
                let branch_type =
                    infer_expr_type_with_ctx(ctx, sources, source_id, &branch.body)?;
                ctx.unify(&result_type, &branch_type)?;
                result_type = ctx.apply(&result_type)?;
            }

            result_type
        }

        // Path expression - look up variable in scope
        ast::Expr::Path(path) => {
            // Try to get the variable name from the path
            if let Some(name) = get_path_ident(path, sources, source_id) {
                ctx.lookup_var(&name)?
            } else {
                ResolvedType::Any
            }
        }

        // Grouped expression (parentheses)
        ast::Expr::Group(group) => {
            infer_expr_type_with_ctx(ctx, sources, source_id, &group.expr)?
        }

        // For most other expressions, return Any (gradual typing)
        _ => ResolvedType::Any,
    })
}

/// Infer the type of a block, tracking variable bindings.
fn infer_block_type_with_ctx(
    ctx: &mut InferenceContext,
    sources: &Sources,
    source_id: SourceId,
    block: &ast::Block,
) -> compile::Result<ResolvedType> {
    ctx.push_scope()?;

    let mut last_type = ResolvedType::Tuple(Vec::new()); // Unit type by default

    for stmt in &block.statements {
        match stmt {
            // Expression statement (no semicolon) - this is the return value if last
            ast::Stmt::Expr(expr) => {
                last_type = infer_expr_type_with_ctx(ctx, sources, source_id, expr)?;
            }
            // Semi statement (with semicolon) - evaluate but result is unit
            ast::Stmt::Semi(semi) => {
                let _ = infer_expr_type_with_ctx(ctx, sources, source_id, &semi.expr)?;
                last_type = ResolvedType::Tuple(Vec::new()); // Unit
            }
            // Local binding (let statement)
            ast::Stmt::Local(local) => {
                // Infer the RHS type
                let expr_type = infer_expr_type_with_ctx(ctx, sources, source_id, &local.expr)?;

                // Bind the pattern variables to the inferred type
                bind_pattern_vars(ctx, sources, source_id, &local.pat, &expr_type)?;

                last_type = ResolvedType::Tuple(Vec::new()); // Let returns unit
            }
            // Item declarations don't affect block type
            ast::Stmt::Item(..) => {}
        }
    }

    ctx.pop_scope();
    Ok(last_type)
}

/// Bind variables from a pattern to a type.
fn bind_pattern_vars(
    ctx: &mut InferenceContext,
    sources: &Sources,
    source_id: SourceId,
    pat: &ast::Pat,
    ty: &ResolvedType,
) -> compile::Result<()> {
    match pat {
        // Simple identifier binding
        ast::Pat::Path(path) => {
            if let Some(name) = get_path_ident(&path.path, sources, source_id) {
                ctx.bind_var(name, ty.try_clone()?)?;
            }
        }
        // Tuple pattern - destructure
        ast::Pat::Tuple(tuple) => {
            if let ResolvedType::Tuple(types) = ty {
                for (i, (item, _)) in tuple.items.iter().enumerate() {
                    let item_type = if let Some(t) = types.get(i) {
                        t.try_clone()?
                    } else {
                        ResolvedType::Variable(ctx.fresh_var())
                    };
                    bind_pattern_vars(ctx, sources, source_id, item, &item_type)?;
                }
            }
        }
        // Binding pattern (name @ pattern)
        ast::Pat::Binding(binding) => {
            if let Some(name) = sources.source(source_id, binding.key.span()) {
                ctx.bind_var(String::try_from(name)?, ty.try_clone()?)?;
            }
            // Also bind the inner pattern
            bind_pattern_vars(ctx, sources, source_id, &binding.pat, ty)?;
        }
        // Ignore pattern (_)
        ast::Pat::Ignore(_) => {}
        // Other patterns - don't bind anything for now
        _ => {}
    }
    Ok(())
}

/// Get the identifier name from a simple path expression.
fn get_path_ident(path: &ast::Path, sources: &Sources, source_id: SourceId) -> Option<String> {
    // Only handle simple single-segment paths (local variables)
    if path.global.is_some() || !path.rest.is_empty() {
        return None;
    }

    if let ast::PathSegment::Ident(ident) = &path.first {
        sources
            .source(source_id, ident.span)
            .and_then(|s| String::try_from(s).ok())
    } else {
        None
    }
}

/// Infer the type of an expression (simple version without context).
fn infer_expr_type(expr: &ast::Expr) -> compile::Result<ResolvedType> {
    Ok(match expr {
        ast::Expr::Lit(lit) => ResolvedType::from_literal(&lit.lit)?,
        ast::Expr::Tuple(tuple) => {
            let mut types = Vec::new();
            for (e, _) in tuple.items.iter() {
                types.try_push(infer_expr_type(e)?)?;
            }
            ResolvedType::Tuple(types)
        }
        // For most expressions, we don't know the type without more context
        // In gradual typing, unknown types are treated as Any
        _ => ResolvedType::Any,
    })
}

/// Emit a type mismatch warning or error.
fn emit_type_mismatch(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    span: ast::Span,
    expected: &ResolvedType,
    actual: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    let expected_str = expected.to_display_string()?;
    let actual_str = actual.to_display_string()?;

    if options.strict_types {
        // In strict mode, emit an error
        return Err(compile::Error::msg(
            span,
            format!("Type mismatch: expected `{expected_str}`, found `{actual_str}`"),
        ));
    }

    // In non-strict mode, emit a warning
    q.diagnostics.warning(
        source_id,
        WarningDiagnosticKind::TypeMismatch {
            span,
            expected: expected_str,
            actual: actual_str,
        },
    )?;

    Ok(())
}
