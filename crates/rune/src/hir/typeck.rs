//! Type checking during HIR lowering for gradual typing.
//!
//! This module implements type validation integrated into the HIR lowering process.
//! Type checking happens as expressions are lowered, providing:
//!
//! - Single AST walk (better performance)
//! - Type information available during lowering
//! - Direct access to Query for hash→name resolution
//!
//! Gradual typing semantics:
//! - Functions with return type annotations are checked against their body
//! - Untyped code is treated as having type `Any` and bypasses checking
//! - Type mismatches produce warnings by default, errors in strict mode
//!
//! Type inference:
//! - Type variables for inference
//! - Unification algorithm
//! - Variable binding tracking

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, String, Vec};
use crate::ast::{self, NumberSource, Spanned};
use crate::compile::{self, Options};
use crate::query::{Query, Used};
use crate::{Hash, SourceId, Sources};

use once_cell::sync::Lazy;

// ============================================================================
// Builtin Type Hash Cache
// ============================================================================

/// Cache of builtin type hashes to avoid repeated computation.
/// Maps type name to its hash value.
static BUILTIN_TYPE_HASHES: Lazy<std::collections::HashMap<&'static str, u64>> = Lazy::new(|| {
    let mut map = std::collections::HashMap::new();

    // Helper to compute hash - these paths are known to be valid
    let hash = |name: &str| -> u64 {
        let item = crate::ItemBuf::with_crate("std")
            .expect("std crate path should be valid")
            .extended(name)
            .expect("builtin type name should be valid");
        Hash::type_hash(&item).into_inner()
    };

    map.insert("i64", hash("i64"));
    map.insert("i32", hash("i32"));
    map.insert("i16", hash("i16"));
    map.insert("i8", hash("i8"));
    map.insert("u64", hash("u64"));
    map.insert("u32", hash("u32"));
    map.insert("u16", hash("u16"));
    map.insert("u8", hash("u8"));
    map.insert("f64", hash("f64"));
    map.insert("f32", hash("f32"));
    map.insert("bool", hash("bool"));
    map.insert("char", hash("char"));
    map.insert("String", hash("String"));
    map.insert("Bytes", hash("Bytes"));

    map
});

/// Reverse mapping: hash value to type name for display purposes.
static BUILTIN_HASH_NAMES: Lazy<std::collections::HashMap<u64, &'static str>> = Lazy::new(|| {
    BUILTIN_TYPE_HASHES
        .iter()
        .map(|(&k, &v)| (v, k))
        .collect()
});

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
    /// A named type (e.g., `i64`, `String`, `foo::Bar`) identified by hash
    Named(Hash),
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
            ResolvedType::Named(hash) => ResolvedType::Named(*hash),
            ResolvedType::Tuple(types) => ResolvedType::Tuple(types.try_clone()?),
            ResolvedType::Never => ResolvedType::Never,
            ResolvedType::Any => ResolvedType::Any,
            ResolvedType::Variable(v) => ResolvedType::Variable(*v),
        })
    }
}

impl ResolvedType {
    /// Convert AST type to resolved type.
    pub(crate) fn from_ast_type(
        ty: &ast::Type,
        q: &mut Query<'_, '_>,
        _source_id: SourceId,
    ) -> compile::Result<Self> {
        match ty {
            ast::Type::Path(path) => {
                // Resolve the path to get its hash
                let named = q.convert_path(path)?;

                // Try to query metadata first
                if let Some(meta) = q.query_meta(path, named.item, Used::Used)? {
                    Ok(ResolvedType::Named(meta.hash))
                } else {
                    // If metadata doesn't exist, it might be a built-in type
                    // Compute the hash directly from the item
                    let item = q.pool.item(named.item);
                    let computed_hash = Hash::type_hash(item);

                    // Check if this is a builtin type and use cached hash for consistency
                    if let Some(last) = item.last() {
                        if let Some(type_name) = last.as_str() {
                            if let Some(&cached_hash) = BUILTIN_TYPE_HASHES.get(type_name) {
                                return Ok(ResolvedType::Named(Hash::new(cached_hash)));
                            }
                        }
                    }

                    Ok(ResolvedType::Named(computed_hash))
                }
            }
            ast::Type::Bang(_) => Ok(ResolvedType::Never),
            ast::Type::Tuple(tuple) => {
                let mut types = Vec::new();
                for (inner_ty, _) in tuple.iter() {
                    types.try_push(Self::from_ast_type(inner_ty, q, _source_id)?)?;
                }
                Ok(ResolvedType::Tuple(types))
            }
        }
    }

    /// Get the type of a literal.
    ///
    /// Uses cached builtin type hashes for performance.
    pub(crate) fn from_literal(lit: &ast::Lit) -> compile::Result<Self> {
        Ok(match lit {
            ast::Lit::Bool(_) => {
                ResolvedType::Named(Hash::new(BUILTIN_TYPE_HASHES["bool"]))
            }
            ast::Lit::Byte(_) => {
                ResolvedType::Named(Hash::new(BUILTIN_TYPE_HASHES["u8"]))
            }
            ast::Lit::Str(_) => {
                ResolvedType::Named(Hash::new(BUILTIN_TYPE_HASHES["String"]))
            }
            ast::Lit::ByteStr(_) => {
                ResolvedType::Named(Hash::new(BUILTIN_TYPE_HASHES["Bytes"]))
            }
            ast::Lit::Char(_) => {
                ResolvedType::Named(Hash::new(BUILTIN_TYPE_HASHES["char"]))
            }
            ast::Lit::Number(num) => {
                // Check if it's a float or integer
                let is_float = match &num.source {
                    NumberSource::Text(text) => text.is_fractional,
                    NumberSource::Synthetic(_) => false,
                };
                let hash = if is_float {
                    BUILTIN_TYPE_HASHES["f64"]
                } else {
                    BUILTIN_TYPE_HASHES["i64"]
                };
                ResolvedType::Named(Hash::new(hash))
            }
        })
    }

    /// Convert to display string for error messages.
    pub(crate) fn to_display_string(&self, q: &Query<'_, '_>) -> compile::Result<String> {
        self.to_display_string_impl(q)
    }

    /// Implementation of to_display_string.
    /// Separated to avoid clippy's only_used_in_recursion warning.
    /// Note: `q` is intentionally only used in recursive calls (for Tuple types).
    #[allow(clippy::only_used_in_recursion)]
    fn to_display_string_impl(&self, q: &Query<'_, '_>) -> compile::Result<String> {
        use crate::alloc::fmt::TryWrite;

        Ok(match self {
            ResolvedType::Named(hash) => {
                let hash_value = hash.into_inner();

                // Fast lookup in cached builtin types
                if let Some(&name) = BUILTIN_HASH_NAMES.get(&hash_value) {
                    return Ok(String::try_from(name)?);
                }

                // Fallback to hex for unknown types
                let mut s = String::new();
                write!(s, "0x{:x}", hash_value)?;
                s
            }
            ResolvedType::Tuple(types) => {
                let mut result = String::new();
                result.try_push('(')?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        result.try_push_str(", ")?;
                    }
                    result.try_push_str(&ty.to_display_string_impl(q)?)?;
                }
                result.try_push(')')?;
                result
            }
            ResolvedType::Never => String::try_from("!")?,
            ResolvedType::Any => String::try_from("Any")?,
            ResolvedType::Variable(v) => {
                let mut result = String::new();
                result.try_push_str("?T")?;
                write!(result, "{}", v.0)?;
                result
            }
        })
    }

    /// Check if two types are compatible under gradual typing semantics.
    ///
    /// Returns `true` if the types are compatible (no warning needed).
    pub(crate) fn is_compatible_with(&self, other: &Self) -> bool {
        // Any is compatible with everything
        if matches!(self, ResolvedType::Any) || matches!(other, ResolvedType::Any) {
            return true;
        }

        // Type variables are compatible with everything (they'll be resolved later)
        if matches!(self, ResolvedType::Variable(_)) || matches!(other, ResolvedType::Variable(_)) {
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
    pub(crate) fn new() -> alloc::Result<Self> {
        let mut scopes = Vec::new();
        scopes.try_push(HashMap::new())?;
        Ok(Self {
            next_var: 0,
            substitutions: HashMap::new(),
            scopes,
        })
    }

    /// Create a fresh type variable.
    pub(crate) fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Push a new variable scope.
    pub(crate) fn push_scope(&mut self) -> alloc::Result<()> {
        self.scopes.try_push(HashMap::new())
    }

    /// Pop the current variable scope.
    /// Never pops the global scope (index 0).
    pub(crate) fn pop_scope(&mut self) {
        // Never pop the global scope
        if self.scopes.len() > 1 {
            self.scopes.pop();
        } else {
            // This would be a programming error - should never happen
            debug_assert!(
                false,
                "Attempted to pop global scope in type checker - this is a bug"
            );
        }
    }

    /// Bind a variable name to a type in the current scope.
    pub(crate) fn bind_var(&mut self, name: String, ty: ResolvedType) -> alloc::Result<()> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.try_insert(name, ty)?;
        }
        Ok(())
    }

    /// Look up a variable's type by name.
    pub(crate) fn lookup_var(&self, name: &str) -> compile::Result<ResolvedType> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.try_clone()?);
            }
        }
        // Unknown variable - return Any for gradual typing
        Ok(ResolvedType::Any)
    }

    /// Maximum recursion depth to prevent stack overflow on pathological types.
    const MAX_RECURSION_DEPTH: usize = 128;

    /// Apply substitutions to resolve a type.
    ///
    /// Recursively replaces type variables with their substituted values.
    pub(crate) fn apply(&self, ty: &ResolvedType) -> compile::Result<ResolvedType> {
        self.apply_with_depth(ty, 0)
    }

    /// Apply substitutions with recursion depth tracking.
    fn apply_with_depth(
        &self,
        ty: &ResolvedType,
        depth: usize,
    ) -> compile::Result<ResolvedType> {
        if depth > Self::MAX_RECURSION_DEPTH {
            return Err(compile::Error::new(
                ast::Span::empty(),
                compile::ErrorKind::Custom {
                    error: anyhow::anyhow!("Type recursion limit exceeded - possible infinite type"),
                },
            ));
        }

        Ok(match ty {
            ResolvedType::Variable(v) => {
                if let Some(resolved) = self.substitutions.get(v) {
                    // Recursively apply in case the substitution contains more variables
                    self.apply_with_depth(resolved, depth + 1)?
                } else {
                    // Unresolved type variable - default to Any for gradual typing
                    ResolvedType::Any
                }
            }
            ResolvedType::Tuple(types) => {
                let mut resolved = Vec::new();
                for t in types {
                    resolved.try_push(self.apply_with_depth(t, depth + 1)?)?;
                }
                ResolvedType::Tuple(resolved)
            }
            other => other.try_clone()?,
        })
    }

    /// Unify two types, updating the substitution map.
    ///
    /// Returns Ok(()) if unification succeeds, potentially adding new substitutions.
    pub(crate) fn unify(&mut self, t1: &ResolvedType, t2: &ResolvedType) -> compile::Result<()> {
        let t1 = self.apply(t1)?;
        let t2 = self.apply(t2)?;

        match (&t1, &t2) {
            // Type variable unification - bind to other type
            (ResolvedType::Variable(v), other) | (other, ResolvedType::Variable(v)) => {
                if !occurs_check(*v, other) {
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

    /// Emit a type mismatch diagnostic.
    pub(crate) fn emit_type_mismatch(
        &self,
        q: &mut Query<'_, '_>,
        source_id: SourceId,
        span: &dyn ast::Spanned,
        expected: &ResolvedType,
        actual: &ResolvedType,
        options: &Options,
    ) -> compile::Result<()> {
        use crate::diagnostics::WarningDiagnosticKind;

        let expected_str = expected.to_display_string(q)?;
        let actual_str = actual.to_display_string(q)?;

        if options.strict_types {
            // In strict mode, emit an error
            return Err(compile::Error::msg(
                span.span(),
                format!("Type mismatch: expected `{expected_str}`, found `{actual_str}`"),
            ));
        }

        // In non-strict mode, emit a warning
        q.diagnostics.warning(
            source_id,
            WarningDiagnosticKind::TypeMismatch {
                span: span.span(),
                expected: expected_str,
                actual: actual_str,
            },
        )?;

        Ok(())
    }
}

/// Occurs check to prevent infinite types.
///
/// Returns true if the type variable occurs in the type.
fn occurs_check(var: TypeVar, ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Variable(v) => var == *v,
        ResolvedType::Tuple(types) => types.iter().any(|t| occurs_check(var, t)),
        _ => false,
    }
}

/// Check a function for type mismatches if it has a return type annotation.
///
/// This is called during HIR lowering to integrate type checking into the compilation pipeline.
/// Returns Ok(()) if no type errors are found (or function has no annotations).
pub(crate) fn check_function_if_annotated(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    ast: &ast::ItemFn,
    options: &Options,
) -> compile::Result<()> {
    use crate::alloc::String;

    // Check if there's a return type annotation
    let Some((_, return_type)) = &ast.output else {
        return Ok(()); // No return type annotation, nothing to check
    };

    let expected_type = ResolvedType::from_ast_type(return_type, q, source_id)?;

    // Create an inference context for type inference within this function
    let mut ctx = InferenceContext::new()?;

    // Register parameter types if annotated
    for (arg, _) in ast.args.iter() {
        match arg {
            ast::FnArg::Typed(typed) => {
                // Get parameter name from pattern
                if let Some(name) = extract_pat_name(&typed.pat, q.sources, source_id) {
                    let param_type = ResolvedType::from_ast_type(&typed.ty, q, source_id)?;
                    ctx.bind_var(name, param_type)?;
                }
            }
            ast::FnArg::Pat(pat) => {
                // Untyped parameter - bind as Any
                if let Some(name) = extract_pat_name(pat, q.sources, source_id) {
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
    let inferred_type = infer_block_type_with_ctx(&mut ctx, q.sources, source_id, &ast.body)?;
    let actual_type = ctx.apply(&inferred_type)?;

    // Check if the inferred type matches the expected return type
    if !actual_type.is_compatible_with(&expected_type) {
        ctx.emit_type_mismatch(
            q,
            source_id,
            &ast.body,
            &expected_type,
            &actual_type,
            options,
        )?;
    }

    // Also check for explicit return statements
    check_block_return_type(&mut ctx, q, source_id, &ast.body, &expected_type, options)?;

    Ok(())
}

/// Extract a variable name from a pattern.
fn extract_pat_name(
    pat: &ast::Pat,
    sources: &Sources,
    source_id: SourceId,
) -> Option<alloc::String> {
    match pat {
        ast::Pat::Path(path) => get_path_ident(&path.path, sources, source_id),
        ast::Pat::Binding(binding) => sources
            .source(source_id, binding.key.span())
            .and_then(|s| alloc::String::try_from(s).ok()),
        _ => None,
    }
}

/// Check that explicit return statements in the block match the expected type.
fn check_block_return_type(
    ctx: &mut InferenceContext,
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    block: &ast::Block,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    for stmt in &block.statements {
        match stmt {
            ast::Stmt::Expr(expr) => {
                // Check for explicit returns within the expression
                check_expr_for_returns(ctx, q, source_id, expr, expected, options)?;
            }
            ast::Stmt::Semi(semi) => {
                // Check for explicit returns within the expression
                check_expr_for_returns(ctx, q, source_id, &semi.expr, expected, options)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check if an expression's type matches the expected type.
fn check_expr_type(
    ctx: &mut InferenceContext,
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    expr: &ast::Expr,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    let actual = infer_expr_type_with_ctx(ctx, q.sources, source_id, expr)?;

    if !actual.is_compatible_with(expected) {
        ctx.emit_type_mismatch(q, source_id, expr, expected, &actual, options)?;
    }

    Ok(())
}

/// Check an expression tree for return statements.
fn check_expr_for_returns(
    ctx: &mut InferenceContext,
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    expr: &ast::Expr,
    expected: &ResolvedType,
    options: &Options,
) -> compile::Result<()> {
    match expr {
        ast::Expr::Return(ret) => {
            if let Some(ret_expr) = &ret.expr {
                check_expr_type(ctx, q, source_id, ret_expr, expected, options)?;
            }
        }
        ast::Expr::Block(block) => {
            check_block_return_type(ctx, q, source_id, &block.block, expected, options)?;
        }
        ast::Expr::If(if_expr) => {
            check_block_return_type(ctx, q, source_id, &if_expr.block, expected, options)?;
            for branch in &if_expr.expr_else_ifs {
                check_block_return_type(ctx, q, source_id, &branch.block, expected, options)?;
            }
            if let Some(else_branch) = &if_expr.expr_else {
                check_block_return_type(ctx, q, source_id, &else_branch.block, expected, options)?;
            }
        }
        ast::Expr::Match(match_expr) => {
            for (branch, _) in &match_expr.branches {
                check_expr_for_returns(ctx, q, source_id, &branch.body, expected, options)?;
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
                | ast::BinOp::Gte(_) => {
                    use crate::ItemBuf;
                    let item = ItemBuf::with_item(["bool"])?;
                    ResolvedType::Named(Hash::type_hash(&item))
                }
                // Logical operations - return bool
                ast::BinOp::And(_) | ast::BinOp::Or(_) => {
                    use crate::ItemBuf;
                    let item = ItemBuf::with_item(["bool"])?;
                    ResolvedType::Named(Hash::type_hash(&item))
                }
                // Other operations - return Any
                _ => ResolvedType::Any,
            }
        }

        // Unary operations
        ast::Expr::Unary(unary) => {
            let _operand = infer_expr_type_with_ctx(ctx, sources, source_id, &unary.expr)?;
            match &unary.op {
                ast::UnOp::Not(_) => {
                    use crate::ItemBuf;
                    let item = ItemBuf::with_item(["bool"])?;
                    ResolvedType::Named(Hash::type_hash(&item))
                }
                ast::UnOp::Neg(_) => _operand,
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
                let branch_type = infer_expr_type_with_ctx(ctx, sources, source_id, &branch.body)?;
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
        ast::Expr::Group(group) => infer_expr_type_with_ctx(ctx, sources, source_id, &group.expr)?,

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
                ctx.bind_var(alloc::String::try_from(name)?, ty.try_clone()?)?;
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
fn get_path_ident(
    path: &ast::Path,
    sources: &Sources,
    source_id: SourceId,
) -> Option<alloc::String> {
    // Only handle simple single-segment paths (local variables)
    if path.global.is_some() || !path.rest.is_empty() {
        return None;
    }

    if let ast::PathSegment::Ident(ident) = &path.first {
        sources
            .source(source_id, ident.span)
            .and_then(|s| alloc::String::try_from(s).ok())
    } else {
        None
    }
}

// ============================================================================
// Struct Literal Type Checking
// ============================================================================

/// Check struct literal field assignments against declared field types.
///
/// This is called during HIR lowering when a struct literal is encountered.
/// For each field assignment, we:
/// 1. Look up the expected field type from the struct definition
/// 2. Infer the actual type of the assigned expression
/// 3. Check compatibility and emit warnings/errors
pub(crate) fn check_struct_literal_if_typed_with_item(
    q: &mut Query<'_, '_>,
    source_id: SourceId,
    ast: &ast::ExprObject,
    item_id: compile::ItemId,
    options: &Options,
) -> compile::Result<()> {
    use crate::parse::Resolve;

    // Look up the struct metadata using query_meta
    let Some(meta) = q.query_meta(ast, item_id, Default::default())? else {
        return Ok(()); // Struct not found in metadata - skip checking
    };

    // Extract field_types from the metadata
    let field_types = match &meta.kind {
        crate::compile::meta::Kind::Struct {
            field_types: Some(types),
            ..
        } => types,
        crate::compile::meta::Kind::Struct {
            field_types: None, ..
        } => return Ok(()), // No field types to check
        _ => return Ok(()), // Not a struct - skip checking
    };

    // Clone the field types to avoid borrowing issues
    // (we need to mutably borrow q later for type checking)
    let mut field_types_owned = alloc::Vec::new();
    for (name, ty_opt) in field_types.iter() {
        field_types_owned.try_push((
            name.try_clone()?,
            match ty_opt {
                Some(ty) => Some(ty.try_clone()?),
                None => None,
            },
        ))?;
    }

    // Create a map of field names to their types for quick lookup
    let mut field_type_map = HashMap::new();
    for (name, ty_opt) in &field_types_owned {
        field_type_map.try_insert(name.as_ref(), ty_opt)?;
    }

    // Create resolve context for field names
    let resolve_cx = crate::parse::ResolveContext {
        sources: q.sources,
        storage: q.storage,
    };

    // Pre-resolve all field names and pair them with their assignments
    // This allows us to drop the borrow of q before type checking
    let mut field_checks = alloc::Vec::new();
    for (field, _) in ast.assignments.iter() {
        let field_name = field.key.resolve(resolve_cx)?;

        if let Some(Some(expected_ast_type)) = field_type_map.get(field_name.as_ref()) {
            // Only check fields that have type annotations
            if let Some((_, assigned_expr)) = &field.assign {
                field_checks.try_push((expected_ast_type, assigned_expr))?;
            }
        }
        // Skip fields without type annotations (None in the Option)
    }

    // Now we can type check without holding any immutable borrows
    let mut ctx = InferenceContext::new()?;

    for (expected_ast_type, assigned_expr) in field_checks {
        let expected_type = ResolvedType::from_ast_type(expected_ast_type, q, source_id)?;

        // Infer the type of the assigned expression
        let actual_type = infer_expr_type_with_ctx(&mut ctx, q.sources, source_id, assigned_expr)?;

        let actual_type_resolved = ctx.apply(&actual_type)?;

        // Check if types are compatible
        if !actual_type_resolved.is_compatible_with(&expected_type) {
            ctx.emit_type_mismatch(
                q,
                source_id,
                assigned_expr,
                &expected_type,
                &actual_type_resolved,
                options,
            )?;
        }
    }

    Ok(())
}
