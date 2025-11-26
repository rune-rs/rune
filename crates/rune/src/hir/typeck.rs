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
use crate::alloc::sync::Arc;
use crate::alloc::{self, HashMap, String, Vec};
use crate::ast::{self, NumberSource, Spanned};
use crate::compile::{self, meta, Options};
use crate::query::{Query, Used};
use crate::runtime::Protocol;
use crate::{Context, Hash, SourceId, Sources};

use once_cell::sync::Lazy;

// ============================================================================
// Builtin Type Hash Cache
// ============================================================================

/// Cache of builtin type hashes to avoid repeated computation.
/// Maps type name to its hash value.
static BUILTIN_TYPE_HASHES: Lazy<HashMap<&'static str, u64>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Helper to compute hash - these paths are known to be valid
    let hash = |name: &str| -> u64 {
        let item = crate::ItemBuf::with_crate("std")
            .expect("std crate path should be valid")
            .extended(name)
            .expect("builtin type name should be valid");
        Hash::type_hash(&item).into_inner()
    };

    let types = [
        ("i64", hash("i64")),
        ("i32", hash("i32")),
        ("i16", hash("i16")),
        ("i8", hash("i8")),
        ("u64", hash("u64")),
        ("u32", hash("u32")),
        ("u16", hash("u16")),
        ("u8", hash("u8")),
        ("f64", hash("f64")),
        ("f32", hash("f32")),
        ("bool", hash("bool")),
        ("char", hash("char")),
        ("String", hash("String")),
        ("Bytes", hash("Bytes")),
    ];

    for (name, hash_val) in types {
        map.try_insert(name, hash_val).expect("builtin type hash should be unique");
    }

    map
});

/// Reverse mapping: hash value to type name for display purposes.
static BUILTIN_HASH_NAMES: Lazy<HashMap<u64, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    for (name, &hash_val) in BUILTIN_TYPE_HASHES.iter() {
        map.try_insert(hash_val, *name).expect("builtin hash should be unique");
    }

    map
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
///
/// Uses `Arc<[ResolvedType]>` for tuple types to enable O(1) cloning.
/// This is important because types are frequently cloned during inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    /// A named type (e.g., `i64`, `String`, `foo::Bar`) identified by hash
    Named(Hash),
    /// A tuple of types. Uses Arc for cheap cloning.
    Tuple(Arc<[ResolvedType]>),
    /// The never type `!`
    Never,
    /// Dynamic/untyped - compatible with everything (gradual typing)
    Any,
    /// A type variable (used during inference)
    Variable(TypeVar),
}

impl ResolvedType {
    /// Create the unit type `()`.
    ///
    /// Creates an empty tuple type representing the unit type in Rune.
    #[inline]
    pub(crate) fn unit() -> Self {
        // For unit type, we create an empty Arc<[ResolvedType]>
        // This is a small allocation but happens infrequently
        ResolvedType::Tuple(Arc::try_from(Vec::new()).unwrap())
    }

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
                Ok(ResolvedType::Tuple(Arc::try_from(types)?))
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
    ///
    /// The `q` parameter is used for recursive tuple type formatting.
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn to_display_string(&self, q: &Query<'_, '_>) -> compile::Result<String> {
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
                    // Recursive call uses `q`
                    result.try_push_str(&ty.to_display_string(q)?)?;
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
        // Single pattern match for efficiency - fast paths first
        match (self, other) {
            // Any is compatible with everything (gradual typing)
            (ResolvedType::Any, _) | (_, ResolvedType::Any) => true,
            // Type variables are compatible with everything (resolved later)
            (ResolvedType::Variable(_), _) | (_, ResolvedType::Variable(_)) => true,
            // Never is bottom type - subtype of everything
            (ResolvedType::Never, _) => true,
            // Named types must match exactly
            (ResolvedType::Named(a), ResolvedType::Named(b)) => a == b,
            // Tuple types must have same arity and compatible elements
            (ResolvedType::Tuple(a), ResolvedType::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a.is_compatible_with(b))
            }
            // All other combinations are incompatible
            _ => false,
        }
    }
}

// ============================================================================
// String Interner for Variable Names
// ============================================================================

/// Interned symbol identifier for variable names.
///
/// Using a compact u32 instead of String reduces memory usage and enables
/// fast equality comparison (integer vs string comparison).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SymbolId(u32);

/// Per-function string interner for variable names.
///
/// Deduplicates variable name strings within a function's type checking context.
/// This reduces memory allocations when the same variable name appears multiple
/// times (e.g., in nested scopes or multiple references).
struct StringInterner {
    /// Storage for interned strings
    strings: Vec<String>,
    /// Lookup map from string content to symbol ID
    lookup: HashMap<String, SymbolId>,
}

impl StringInterner {
    /// Create a new empty interner.
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    /// Intern a string, returning its symbol ID.
    ///
    /// If the string was already interned, returns the existing ID.
    /// Otherwise, stores the string and returns a new ID.
    fn intern(&mut self, s: &str) -> alloc::Result<SymbolId> {
        // Check if already interned
        if let Some(&id) = self.lookup.get(s) {
            return Ok(id);
        }

        // New string - allocate and store
        let id = SymbolId(u32::try_from(self.strings.len()).expect("too many interned strings"));
        let owned = String::try_from(s)?;
        self.lookup.try_insert(owned.try_clone()?, id)?;
        self.strings.try_push(owned)?;
        Ok(id)
    }

}

// ============================================================================
// Type Checker
// ============================================================================

/// Type checker for a function during HIR lowering.
///
/// Combines type inference machinery with function-level type checking state.
/// This is stored in `Ctxt` when a function has type annotations, allowing
/// type inference to happen during the lowering pass rather than as a separate
/// AST walk.
pub(crate) struct TypeChecker<'a> {
    // -- Function-level state --
    /// The expected return type from the function signature (if annotated)
    expected_return: Option<ResolvedType>,
    /// Track the last inferred expression type (for implicit returns)
    pub(crate) last_expr_type: ResolvedType,

    // -- Inference machinery --
    /// Counter for generating fresh type variables
    next_var: usize,
    /// Substitution map: TypeVar -> ResolvedType
    substitutions: HashMap<TypeVar, ResolvedType>,
    /// String interner for variable names
    interner: StringInterner,
    /// Variable scope stack for tracking variable types by name.
    ///
    /// Uses `Vec<(SymbolId, ResolvedType)>` instead of `HashMap` for each scope
    /// because typical scopes are small (< 20 variables). Linear search in
    /// contiguous memory is faster than hash lookups for small collections
    /// due to better cache locality and no hashing overhead.
    ///
    /// Variable names are interned to reduce memory usage and enable fast
    /// integer comparison instead of string comparison.
    scopes: Vec<Vec<(SymbolId, ResolvedType)>>,
    /// Reference to the compilation context for protocol lookups
    context: &'a Context,
}

impl<'a> TypeChecker<'a> {
    /// Create a new type checker for a function.
    pub(crate) fn new(
        context: &'a Context,
        expected_return: Option<ResolvedType>,
    ) -> alloc::Result<Self> {
        let mut scopes = Vec::new();
        scopes.try_push(Vec::new())?;
        Ok(Self {
            expected_return,
            last_expr_type: ResolvedType::unit(),
            next_var: 0,
            substitutions: HashMap::new(),
            interner: StringInterner::new(),
            scopes,
            context,
        })
    }

    /// Infer the type of an expression.
    pub(crate) fn infer_expr(
        &mut self,
        sources: &Sources,
        source_id: SourceId,
        expr: &ast::Expr,
    ) -> compile::Result<ResolvedType> {
        infer_expr_type_with_ctx(self, sources, source_id, expr)
    }

    /// Infer the type of a block.
    pub(crate) fn infer_block(
        &mut self,
        sources: &Sources,
        source_id: SourceId,
        block: &ast::Block,
    ) -> compile::Result<ResolvedType> {
        infer_block_type_with_ctx(self, sources, source_id, block)
    }

    /// Check a return expression against the expected type.
    pub(crate) fn check_return(
        &mut self,
        q: &mut Query<'_, '_>,
        source_id: SourceId,
        expr: &ast::Expr,
        options: &Options,
    ) -> compile::Result<()> {
        // Clone expected type to avoid borrow conflict
        let expected = match &self.expected_return {
            Some(e) => e.clone(),
            None => return Ok(()),
        };
        let actual = self.infer_expr(q.sources, source_id, expr)?;
        if !actual.is_compatible_with(&expected) {
            self.emit_type_mismatch(q, source_id, expr, &expected, &actual, options)?;
        }
        Ok(())
    }

    /// Finalize type checking at the end of a function.
    ///
    /// Checks that the inferred body type matches the expected return type.
    pub(crate) fn finalize(
        &mut self,
        q: &mut Query<'_, '_>,
        source_id: SourceId,
        body_span: &dyn Spanned,
        options: &Options,
    ) -> compile::Result<()> {
        if let Some(expected) = &self.expected_return {
            let actual = self.apply(&self.last_expr_type.clone())?;
            if !actual.is_compatible_with(expected) {
                self.emit_type_mismatch(q, source_id, body_span, expected, &actual, options)?;
            }
        }
        Ok(())
    }

    /// Get the compilation context for protocol lookups.
    pub(crate) fn context(&self) -> &Context {
        self.context
    }

    /// Create a fresh type variable.
    pub(crate) fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Push a new variable scope.
    pub(crate) fn push_scope(&mut self) -> alloc::Result<()> {
        self.scopes.try_push(Vec::new())
    }

    /// Pop the current variable scope.
    ///
    /// # Panics
    ///
    /// Panics if called when only the global scope remains. This indicates
    /// a bug in the compiler where scope push/pop calls are unbalanced.
    pub(crate) fn pop_scope(&mut self) {
        assert!(
            self.scopes.len() > 1,
            "Attempted to pop global scope in type checker - this is a compiler bug"
        );
        self.scopes.pop();
    }

    /// Bind a variable name to a type in the current scope.
    ///
    /// Accepts `&str` and interns the name internally. If the variable already
    /// exists in the current scope, its type is updated (shadowing within scope).
    ///
    /// # Panics
    ///
    /// Panics if no scope exists. This should never happen as the constructor
    /// always creates an initial scope.
    pub(crate) fn bind_var(&mut self, name: &str, ty: ResolvedType) -> alloc::Result<()> {
        let name_id = self.interner.intern(name)?;

        let scope = self
            .scopes
            .last_mut()
            .expect("TypeChecker must always have at least one scope");

        // Linear search to check for existing binding (update if found)
        // Uses fast integer comparison instead of string comparison
        for (existing_id, existing_ty) in scope.iter_mut() {
            if *existing_id == name_id {
                *existing_ty = ty;
                return Ok(());
            }
        }

        // New binding
        scope.try_push((name_id, ty))?;
        Ok(())
    }

    /// Look up a variable's type by name.
    ///
    /// Searches from innermost to outermost scope, returning the first match.
    /// Returns `Any` for unknown variables (gradual typing semantics).
    pub(crate) fn lookup_var(&self, name: &str) -> compile::Result<ResolvedType> {
        // Check if this name has been interned - if not, it can't be in scope
        let Some(&name_id) = self.interner.lookup.get(name) else {
            return Ok(ResolvedType::Any);
        };

        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            // Linear search within scope using fast integer comparison
            for (var_id, ty) in scope.iter() {
                if *var_id == name_id {
                    return Ok(ty.clone());
                }
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
    ///
    /// Returns an error if the recursion depth exceeds `MAX_RECURSION_DEPTH`,
    /// which typically indicates an infinite recursive type definition.
    fn apply_with_depth(
        &self,
        ty: &ResolvedType,
        depth: usize,
    ) -> compile::Result<ResolvedType> {
        if depth > Self::MAX_RECURSION_DEPTH {
            // Note: We use Span::empty() here because type resolution happens
            // after AST parsing. The caller should catch this error and report
            // it with the appropriate span from the expression being type-checked.
            return Err(compile::Error::msg(
                ast::Span::empty(),
                "Type recursion limit exceeded. This usually indicates a circular \
                 type reference. Check for recursive type definitions.",
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
                for t in types.iter() {
                    resolved.try_push(self.apply_with_depth(t, depth + 1)?)?;
                }
                ResolvedType::Tuple(Arc::try_from(resolved)?)
            }
            other => other.clone(),
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
                    self.substitutions.try_insert(*v, other.clone())?;
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
/// Returns true if the type variable occurs in the type, which would
/// indicate a recursive type definition like `T = List<T>`.
///
/// # Limitations
///
/// Currently only traverses `Tuple` types. If `ResolvedType::Named` is
/// extended to support generic type parameters in the future, this function
/// must be updated to traverse those as well to detect cycles like `T = Foo<T>`.
fn occurs_check(var: TypeVar, ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Variable(v) => var == *v,
        ResolvedType::Tuple(types) => types.iter().any(|t| occurs_check(var, t)),
        // Named types currently don't have generic parameters, so no traversal needed.
        // Never and Any cannot contain type variables.
        _ => false,
    }
}


/// Look up the return type of a protocol implementation for a given type.
///
/// This uses `Hash::associated_function(type_hash, protocol.hash)` to find
/// the protocol implementation and extract its return type from the signature.
///
/// Returns `None` if:
/// - No protocol implementation is found
/// - The protocol implementation has no signature information
fn lookup_protocol_return_type(
    cx: &Context,
    type_hash: Hash,
    protocol: &Protocol,
) -> Option<ResolvedType> {
    // Look up the associated function for this protocol
    let protocol_hash = Hash::associated_function(type_hash, protocol);

    // Find the metadata for this protocol implementation
    let mut meta_iter = cx.lookup_meta_by_hash(protocol_hash);
    let meta = meta_iter.next()?;

    // Extract the return type from the function signature
    if let meta::Kind::Function { signature, .. } = &meta.kind {
        // Convert TypeHash to ResolvedType
        if signature.return_type.base != Hash::EMPTY {
            return Some(ResolvedType::Named(signature.return_type.base));
        }
    }

    None
}

/// Check if a type hash is one of the built-in arithmetic types (i64, u64, f64).
/// These types have special handling in ArithmeticOps and don't need protocol lookup.
fn is_builtin_arithmetic_type(type_hash: Hash) -> bool {
    // Check against cached builtin hashes
    let hash_value = type_hash.into_inner();
    BUILTIN_TYPE_HASHES
        .get("i64")
        .is_some_and(|&h| h == hash_value)
        || BUILTIN_TYPE_HASHES
            .get("u64")
            .is_some_and(|&h| h == hash_value)
        || BUILTIN_TYPE_HASHES
            .get("f64")
            .is_some_and(|&h| h == hash_value)
}

/// Map a binary operator to its corresponding protocol.
fn binop_to_protocol(op: &ast::BinOp) -> Option<&'static Protocol> {
    Some(match op {
        ast::BinOp::Add(_) => &Protocol::ADD,
        ast::BinOp::Sub(_) => &Protocol::SUB,
        ast::BinOp::Mul(_) => &Protocol::MUL,
        ast::BinOp::Div(_) => &Protocol::DIV,
        ast::BinOp::Rem(_) => &Protocol::REM,
        ast::BinOp::BitAnd(_) => &Protocol::BIT_AND,
        ast::BinOp::BitOr(_) => &Protocol::BIT_OR,
        ast::BinOp::BitXor(_) => &Protocol::BIT_XOR,
        ast::BinOp::Shl(_) => &Protocol::SHL,
        ast::BinOp::Shr(_) => &Protocol::SHR,
        ast::BinOp::Eq(_) | ast::BinOp::Neq(_) => &Protocol::PARTIAL_EQ,
        ast::BinOp::Lt(_) | ast::BinOp::Gt(_) | ast::BinOp::Lte(_) | ast::BinOp::Gte(_) => {
            &Protocol::PARTIAL_CMP
        }
        // Logical operators and assignment operators don't have protocols
        _ => return None,
    })
}

/// Infer the type of an expression using the type checker.
fn infer_expr_type_with_ctx(
    ctx: &mut TypeChecker<'_>,
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
            ResolvedType::Tuple(Arc::try_from(types)?)
        }

        // Binary operations - use protocol lookups for return types
        ast::Expr::Binary(binary) => {
            let lhs = infer_expr_type_with_ctx(ctx, sources, source_id, &binary.lhs)?;
            let rhs = infer_expr_type_with_ctx(ctx, sources, source_id, &binary.rhs)?;
            let applied_lhs = ctx.apply(&lhs)?;

            // For non-builtin types, try protocol lookup for operators
            if let Some(protocol) = binop_to_protocol(&binary.op) {
                if let ResolvedType::Named(type_hash) = &applied_lhs {
                    // Skip protocol lookup for builtin arithmetic types - they have known behavior
                    if !is_builtin_arithmetic_type(*type_hash) {
                        if let Some(return_type) =
                            lookup_protocol_return_type(ctx.context(), *type_hash, protocol)
                        {
                            return Ok(return_type);
                        }
                    }
                }
            }

            // Fallback: builtin types or no protocol implementation found
            match &binary.op {
                // Arithmetic operations - result type same as operands (for builtins)
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
            let operand = infer_expr_type_with_ctx(ctx, sources, source_id, &unary.expr)?;
            let applied_operand = ctx.apply(&operand)?;

            // Try protocol lookup for unary operations on non-builtin types
            let protocol = match &unary.op {
                ast::UnOp::Not(_) => Some(&Protocol::NOT),
                ast::UnOp::Neg(_) => Some(&Protocol::NEG),
                _ => None,
            };

            if let Some(protocol) = protocol {
                if let ResolvedType::Named(type_hash) = &applied_operand {
                    if !is_builtin_arithmetic_type(*type_hash) {
                        if let Some(return_type) =
                            lookup_protocol_return_type(ctx.context(), *type_hash, protocol)
                        {
                            return Ok(return_type);
                        }
                    }
                }
            }

            // Fallback
            match &unary.op {
                ast::UnOp::Not(_) => {
                    use crate::ItemBuf;
                    let item = ItemBuf::with_item(["bool"])?;
                    ResolvedType::Named(Hash::type_hash(&item))
                }
                ast::UnOp::Neg(_) => applied_operand,
                _ => ResolvedType::Any,
            }
        }

        // Block expressions - type is the last expression's type
        ast::Expr::Block(block) => infer_block_type_with_ctx(ctx, sources, source_id, &block.block)?,

        // If expressions - unify all branches
        ast::Expr::If(if_expr) => {
            let then_type = infer_block_type_with_ctx(ctx, sources, source_id, &if_expr.block)?;

            // Check else-if branches
            for branch in &if_expr.expr_else_ifs {
                let branch_type = infer_block_type_with_ctx(ctx, sources, source_id, &branch.block)?;
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
    ctx: &mut TypeChecker<'_>,
    sources: &Sources,
    source_id: SourceId,
    block: &ast::Block,
) -> compile::Result<ResolvedType> {
    ctx.push_scope()?;

    let mut last_type = ResolvedType::unit();

    for stmt in &block.statements {
        match stmt {
            // Expression statement (no semicolon) - this is the return value if last
            ast::Stmt::Expr(expr) => {
                last_type = infer_expr_type_with_ctx(ctx, sources, source_id, expr)?;
            }
            // Semi statement (with semicolon) - evaluate but result is unit
            ast::Stmt::Semi(semi) => {
                let _ = infer_expr_type_with_ctx(ctx, sources, source_id, &semi.expr)?;
                last_type = ResolvedType::unit();
            }
            // Local binding (let statement)
            ast::Stmt::Local(local) => {
                // Infer the RHS type
                let expr_type = infer_expr_type_with_ctx(ctx, sources, source_id, &local.expr)?;

                // Bind the pattern variables to the inferred type
                bind_pattern_vars(ctx, sources, source_id, &local.pat, &expr_type)?;

                last_type = ResolvedType::unit();
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
    ctx: &mut TypeChecker<'_>,
    sources: &Sources,
    source_id: SourceId,
    pat: &ast::Pat,
    ty: &ResolvedType,
) -> compile::Result<()> {
    match pat {
        // Simple identifier binding
        ast::Pat::Path(path) => {
            if let Some(name) = get_path_ident(&path.path, sources, source_id) {
                ctx.bind_var(&name, ty.clone())?;
            }
        }
        // Tuple pattern - destructure
        ast::Pat::Tuple(tuple) => {
            if let ResolvedType::Tuple(types) = ty {
                for (i, (item, _)) in tuple.items.iter().enumerate() {
                    let item_type = if let Some(t) = types.get(i) {
                        t.clone()
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
                ctx.bind_var(name, ty.clone())?;
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
        let cloned_ty = match ty_opt {
            Some(ty) => Some(ty.try_clone()?),
            None => None,
        };
        field_types_owned.try_push((name.try_clone()?, cloned_ty))?;
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
    let mut ctx = TypeChecker::new(q.context, None)?;

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
