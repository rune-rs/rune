# Gradual Typing Implementation: Critical Analysis & Improvement Plan

**Date**: 2025-12-12
**Branch**: `feature/gradual-typing`
**Total Commits**: 31 (from main)
**Status**: Needs refresh and cleanup

---

## Executive Summary

The gradual typing implementation introduces type annotations and checking to Rune while maintaining backwards compatibility. The core architecture is sound, but the implementation shows signs of iterative development with opportunities for consolidation, improved test organization, and enhanced functionality.

**Key Strengths:**
- Well-integrated TypeChecker in HIR lowering context
- Performance-conscious design (Vec scopes, string interning, lazy hash caches)
- Proper gradual typing semantics (Any type compatibility)
- Protocol lookup integration for operator return types

**Key Issues:**
- Dual lowering paths (lowering.rs vs lowering2.rs) with inconsistent type checking
- Test files proliferated without clear organization
- Missing closure and call-site type checking
- Limited type extraction API

---

## Part 1: Critical Analysis

### 1.1 Commit History Analysis

The commit history reveals iterative development patterns:

```
Commits by Type:
- feat:     12 (39%)
- test:      6 (19%)
- refactor:  6 (19%)
- fix:       4 (13%)
- chore:     2 (6%)
- perf:      1 (3%)
```

**Issues Identified:**

1. **Duplicate Initial Commits**: Two identical `feat: add type annotation parsing` commits (0dee73b, e64cb38)
   - These appear to be part of a separate PR for parsing-only
   - Should be clarified in commit messages

2. **Feature Flag Churn**: Added feature flag (23f80606), then removed it (3817a0ce)
   - Per maintainer feedback, feature flags avoided for cargo feature unification issues
   - Good decision but adds noise to history

3. **Multiple "integrate type checking" Commits**: (642567fb, 1caa7d15)
   - Suggests incremental integration that could be squashed

4. **Rename Churn**: DocType → TypeHash rename (25d6a033)
   - Good naming improvement but scattered across commits

### 1.2 Architecture Analysis

#### Type Checking Integration (crates/rune/src/hir/typeck.rs)

**Structure:**
```
TypeChecker
├── Function-level state
│   ├── expected_return: Option<ResolvedType>
│   └── last_expr_type: ResolvedType
├── Inference machinery
│   ├── next_var: usize (type variable counter)
│   ├── substitutions: HashMap<TypeVar, ResolvedType>
│   ├── interner: StringInterner
│   └── scopes: Vec<Vec<(SymbolId, ResolvedType)>>
└── context: &Context (for protocol lookups)
```

**Strengths:**
- Vec-based scope storage optimized for small scopes (line 354)
- String interning reduces allocations (lines 276-322)
- Lazy static builtin type hashes (lines 37-82)
- Proper occurs check for recursive types (lines 656-664)

**Issues:**

| Issue | Location | Impact | Severity |
|-------|----------|--------|----------|
| lowering2.rs lacks TypeChecker | lowering2.rs | Some code paths skip type checking | High |
| No closure type checking | lowering.rs:203-268 | Closures bypass type checking | Medium |
| No call-site argument checking | typeck.rs | Only return types checked | Medium |
| to_display_string uses Query but recursion doesn't | typeck.rs:206 | Clippy warning suppressed | Low |

#### HIR Lowering Integration

**File: crates/rune/src/hir/lowering.rs**

Type checking entry point (lines 43-92):
```rust
pub(crate) fn item_fn<'hir>(...) -> compile::Result<hir::ItemFn<'hir>> {
    // Set up type checking if function has type annotations
    if let Some((_, return_type)) = &ast.output {
        let expected = ResolvedType::from_ast_type(return_type, &mut cx.q, source_id)?;
        let mut typeck = TypeChecker::new(cx.q.context, Some(expected))?;
        // Register parameter types...
        cx.typeck = Some(typeck);
    }

    // Lower function...

    // Finalize type checking
    if let Some(ref mut typeck) = cx.typeck {
        typeck.finalize(&mut cx.q, source_id, &ast.body, options)?;
    }
}
```

**Problem**: `lowering2.rs` is a separate lowering path that doesn't call `TypeChecker`. This means code using the newer lowering path skips type checking entirely.

#### Type Extraction API (crates/rune/src/runtime/unit.rs)

**API Surface (lines 242-365):**
```rust
impl<S> Unit<S> {
    pub fn function_signatures(&self) -> impl Iterator<Item = FunctionSignature>
    pub fn function_signature(&self, hash: Hash) -> Option<FunctionSignature>
    pub fn function_signature_by_name(&self, name: &str) -> Option<FunctionSignature>
}
```

**Limitations:**
- No way to query struct field types at runtime
- No way to query local variable types
- Type information comes from debug info (not always present)
- Nested tuple parsing in `parse_type_string` is simplistic (line 379)

### 1.3 Test Suite Analysis

**Test Files:**
```
gradual_typing.rs                  - 881 lines (core acceptance tests)
gradual_typing_inference.rs        - 468 lines (inference tests)
gradual_typing_integration.rs      -  63 lines (integration tests)
gradual_typing_complex_scenarios.rs - 388 lines (complex scenarios)
gradual_typing_edge_cases.rs       - 221 lines (edge cases)
gradual_typing_errors.rs           - 296 lines (error handling)
gradual_typing_performance.rs      - 308 lines (performance tests)
gradual_typing_protocols.rs        - 309 lines (protocol tests)
type_extraction.rs                 - 369 lines (API tests)
type_extraction_comprehensive.rs   -  68 lines (comprehensive API)
```

**Issues:**
1. **Fragmentation**: 10 files with overlapping concerns
2. **Naming**: `type_extraction_comprehensive.rs` at 68 lines isn't very comprehensive
3. **Missing coverage**:
   - No closure type checking tests (because not implemented)
   - Limited negative test cases for type extraction
   - No fuzz testing for parser edge cases

### 1.4 Naming & Convention Issues

**Good:**
- `TypeChecker` follows pattern of other checkers
- `ResolvedType` clearly indicates resolved state
- Commit messages follow conventional commits

**Issues:**
- `check_struct_literal_if_typed_with_item` - verbose name (line 1015)
- `infer_expr_type_with_ctx` vs `infer_expr` inconsistent (line 737 vs 382)
- Some test functions lack `#[doc]` comments

---

## Part 2: Improvement Plan

### Task Dependency Graph

```
[P1: Foundation]
    ├── T1.1: Consolidate lowering paths
    ├── T1.2: Add closure type checking
    └── T1.3: Fix clippy warnings

[P2: Testing] (depends on P1)
    ├── T2.1: Consolidate test files
    ├── T2.2: Add missing coverage
    └── T2.3: Add documentation

[P3: API Enhancement] (parallel with P2)
    ├── T3.1: Extend type extraction API
    ├── T3.2: Improve error messages
    └── T3.3: Add call-site checking (optional)

[P4: Cleanup] (depends on P2, P3)
    ├── T4.1: Squash/rebase commits
    ├── T4.2: Update examples
    └── T4.3: Final documentation
```

---

## Part 3: Detailed Task Specifications

### P1: Foundation Tasks

#### T1.1: Consolidate Lowering Paths

**Priority**: High
**Complexity**: Medium
**Files**: `lowering.rs`, `lowering2.rs`, `ctxt.rs`
**Parallel**: Yes (independent)

**Problem**: `lowering2.rs` uses a different code path that bypasses `TypeChecker`.

**Analysis Required**:
```bash
# Find which code uses lowering2
grep -r "lowering2::" crates/rune/src/
```

**Implementation**:

Option A: Add TypeChecker to lowering2.rs (recommended)
```rust
// crates/rune/src/hir/lowering2.rs

// At top, add import:
use crate::hir::typeck::{TypeChecker, ResolvedType};

// In item_fn function (around line 47):
pub(crate) fn item_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    is_instance: bool,
) -> Result<hir::ItemFn<'hir>> {
    // ... existing code ...

    // ADD: Set up type checking if return type present
    // Parse return type annotation from stream if present
    if let Some(return_type) = parse_return_type_annotation(p)? {
        let expected = ResolvedType::from_ast_type(&return_type, &mut cx.q, cx.source_id)?;
        let mut typeck = TypeChecker::new(cx.q.context, Some(expected))?;
        // Register parameter types
        cx.typeck = Some(typeck);
    }

    // ... rest of function ...

    // ADD: Finalize type checking
    if let Some(ref mut typeck) = cx.typeck {
        // Need to reconstruct AST for inference, or track during lowering
        typeck.finalize(&mut cx.q, cx.source_id, /* span */, cx.q.options)?;
    }
    cx.typeck = None;
}
```

Option B: Unify into single lowering path (more work, cleaner result)

**Verification**:
```bash
cargo test gradual_typing
cargo clippy --all-features
```

---

#### T1.2: Add Closure Type Checking

**Priority**: High
**Complexity**: Medium
**Files**: `lowering.rs`, `typeck.rs`
**Parallel**: Yes (independent of T1.1)

**Problem**: Closures bypass type checking even when they have type annotations.

**Current Code** (lowering.rs:203-268):
```rust
fn expr_call_closure<'hir>(...) -> compile::Result<hir::ExprKind<'hir>> {
    // ... no type checking happens here
    let args = iter!(ast.args.as_slice(), |(arg, _)| fn_arg(cx, arg)?);
    let body = alloc!(expr(cx, &ast.body)?);
    // ...
}
```

**Implementation**:
```rust
// crates/rune/src/hir/lowering.rs

fn expr_call_closure<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprClosure,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    // ... existing item/meta lookup ...

    // NEW: Set up type checking for closure if it has type annotations
    let closure_has_types = ast.args.iter().any(|(arg, _)| matches!(arg, ast::FnArg::Typed(_)));
    let prev_typeck = cx.typeck.take();

    if closure_has_types {
        // Note: Closures don't have explicit return type syntax in Rune
        // We could infer it from the body, but for now just check params
        let mut typeck = TypeChecker::new(cx.q.context, None)?;

        for (arg, _) in ast.args.as_slice() {
            register_fn_arg_type(&mut typeck, &mut cx.q, cx.source_id, arg)?;
        }

        cx.typeck = Some(typeck);
    }

    cx.scopes.push_captures()?;
    let args = iter!(ast.args.as_slice(), |(arg, _)| fn_arg(cx, arg)?);
    let body = alloc!(expr(cx, &ast.body)?);
    let layer = cx.scopes.pop().with_span(&ast.body)?;

    // NEW: Restore previous type checker
    cx.typeck = prev_typeck;

    // ... rest unchanged ...
}
```

**Test**:
```rust
// crates/rune/src/tests/gradual_typing.rs

/// Closures with typed parameters should warn on type mismatch
#[test]
fn warn_closure_param_type_mismatch() {
    assert_warnings! {
        r#"
        let add = |a: i64, b: i64| { a + b };
        add("not", "numbers")
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}
```

---

#### T1.3: Fix Clippy Warnings

**Priority**: Low
**Complexity**: Low
**Files**: `typeck.rs`
**Parallel**: Yes

**Issue**: `only_used_in_recursion` suppressed at line 206.

**Current**:
```rust
#[allow(clippy::only_used_in_recursion)]
pub(crate) fn to_display_string(&self, q: &Query<'_, '_>) -> compile::Result<String> {
```

**Fix**:
```rust
pub(crate) fn to_display_string(&self, q: &Query<'_, '_>) -> compile::Result<String> {
    self.to_display_string_inner(q)
}

fn to_display_string_inner(&self, q: &Query<'_, '_>) -> compile::Result<String> {
    // ... existing implementation using q for recursive calls
}
```

Or remove `q` parameter if truly unused in non-recursive cases and pass it only for tuple recursion.

---

### P2: Testing Tasks

#### T2.1: Consolidate Test Files

**Priority**: Medium
**Complexity**: Low
**Files**: All `gradual_typing_*.rs` files
**Parallel**: Yes

**Current Structure** (10 files, fragmented):
```
gradual_typing.rs                  (881 lines)
gradual_typing_inference.rs        (468 lines)
gradual_typing_integration.rs      (63 lines)
gradual_typing_complex_scenarios.rs (388 lines)
gradual_typing_edge_cases.rs       (221 lines)
gradual_typing_errors.rs           (296 lines)
gradual_typing_performance.rs      (308 lines)
gradual_typing_protocols.rs        (309 lines)
type_extraction.rs                 (369 lines)
type_extraction_comprehensive.rs   (68 lines)
```

**Proposed Structure** (4 files, organized):
```
gradual_typing/
├── mod.rs                     # Module declaration + common helpers
├── basic.rs                   # Core acceptance tests (from gradual_typing.rs)
├── inference.rs               # Type inference tests (keep as-is)
├── diagnostics.rs             # Errors + warnings (merge errors.rs + edge_cases.rs)
├── protocols.rs               # Protocol-based type checking (keep)
└── extraction.rs              # Type extraction API (merge type_extraction*.rs)
```

**Implementation**:

1. Create directory structure:
```bash
mkdir -p crates/rune/src/tests/gradual_typing
```

2. Create mod.rs:
```rust
// crates/rune/src/tests/gradual_typing/mod.rs

//! Tests for gradual typing support.
//!
//! ## Test Organization
//!
//! - `basic`: Core acceptance tests for type annotations
//! - `inference`: Type inference and unification tests
//! - `diagnostics`: Error messages and warnings
//! - `protocols`: Protocol-based operator typing
//! - `extraction`: Runtime type extraction API

mod basic;
mod diagnostics;
mod extraction;
mod inference;
mod protocols;

// Re-export test helpers
pub(super) use crate::tests::prelude::*;
```

3. Move tests maintaining original functionality.

4. Update `tests.rs`:
```rust
// crates/rune/src/tests.rs

mod gradual_typing;  // Now a directory module
// Remove: mod gradual_typing_*;
// Remove: mod type_extraction*;
```

---

#### T2.2: Add Missing Test Coverage

**Priority**: Medium
**Complexity**: Medium
**Parallel**: After T2.1

**Missing Coverage Areas**:

1. **Closure type checking** (pending T1.2):
```rust
#[test]
fn closure_typed_params_infer_body() {
    let result: i64 = rune! {
        let f = |x: i64| x * 2;
        f(21)
    };
    assert_eq!(result, 42);
}

#[test]
fn closure_typed_nested() {
    let result: i64 = rune! {
        let outer = |a: i64| {
            let inner = |b: i64| a + b;
            inner(10)
        };
        outer(5)
    };
    assert_eq!(result, 15);
}
```

2. **Negative type extraction tests**:
```rust
#[test]
fn type_extraction_missing_debug_info() {
    // Compile without debug info
    let unit = compile_without_debug("fn foo() {}");
    assert!(unit.function_signature_by_name("foo").is_none());
}

#[test]
fn type_extraction_nonexistent_function() {
    let unit = compile_helper("fn foo() {}", &mut Diagnostics::new());
    assert!(unit.function_signature_by_name("bar").is_none());
}
```

3. **Parser edge cases**:
```rust
#[test]
fn parse_deeply_nested_tuple() {
    let _: () = rune! {
        fn nested() -> ((i64, i64), (String, bool)) {
            ((1, 2), ("a", true))
        }
        nested();
    };
}

#[test]
fn parse_type_with_generics_future() {
    // When generics are supported
    // fn generic<T>(x: T) -> T { x }
}
```

---

#### T2.3: Add Documentation

**Priority**: Low
**Complexity**: Low
**Parallel**: Yes

**Files needing documentation**:

1. `typeck.rs` - Add module-level examples:
```rust
//! # Examples
//!
//! Type checking is automatic for functions with type annotations:
//!
//! ```rune
//! fn add(a: i64, b: i64) -> i64 {
//!     a + b  // Checked: must return i64
//! }
//! ```
//!
//! Mixed typed/untyped (gradual typing):
//!
//! ```rune
//! fn process(typed: i64, untyped) -> i64 {
//!     typed + untyped  // untyped treated as Any
//! }
//! ```
```

2. `ResolvedType` - Document variants:
```rust
/// Resolved type information for gradual type checking.
///
/// # Variants
///
/// - `Named(Hash)` - A named type identified by its hash (e.g., `i64`, `String`)
/// - `Tuple(Arc<[ResolvedType]>)` - A tuple type with element types
/// - `Never` - The never type `!`, subtype of all types
/// - `Any` - Dynamic type, compatible with everything (gradual typing escape hatch)
/// - `Variable(TypeVar)` - Inference placeholder, resolved during unification
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedType { ... }
```

---

### P3: API Enhancement Tasks

#### T3.1: Extend Type Extraction API

**Priority**: Medium
**Complexity**: Medium
**Files**: `unit.rs`, `type_info.rs`
**Parallel**: Yes

**Current API** (limited):
```rust
impl<S> Unit<S> {
    pub fn function_signatures(&self) -> impl Iterator<Item = FunctionSignature>
    pub fn function_signature(&self, hash: Hash) -> Option<FunctionSignature>
    pub fn function_signature_by_name(&self, name: &str) -> Option<FunctionSignature>
}
```

**Proposed Extensions**:

```rust
// crates/rune/src/runtime/unit.rs

impl<S> Unit<S> {
    // EXISTING
    pub fn function_signatures(&self) -> impl Iterator<Item = FunctionSignature> { ... }
    pub fn function_signature(&self, hash: Hash) -> Option<FunctionSignature> { ... }
    pub fn function_signature_by_name(&self, name: &str) -> Option<FunctionSignature> { ... }

    // NEW: Struct type information

    /// Get type information for a struct by its hash.
    ///
    /// Returns field names and their types (if annotated).
    ///
    /// # Example
    ///
    /// ```rust
    /// let unit = rune::prepare("struct Point { x: i64, y: i64 }").build()?;
    /// if let Some(info) = unit.struct_info_by_name("Point") {
    ///     for field in &info.fields {
    ///         println!("{}: {:?}", field.name, field.type_info);
    ///     }
    /// }
    /// ```
    pub fn struct_info(&self, hash: Hash) -> Option<StructInfo> {
        let debug = self.debug_info()?;
        // Look up in RTTI
        let rtti = self.logic.rtti.get(&hash)?;
        self.build_struct_info(rtti)
    }

    /// Get struct info by name (last path component).
    pub fn struct_info_by_name(&self, name: &str) -> Option<StructInfo> {
        for (hash, rtti) in self.logic.rtti.iter() {
            if rtti.item.last().and_then(|c| c.as_str()) == Some(name) {
                return self.build_struct_info(rtti);
            }
        }
        None
    }

    fn build_struct_info(&self, rtti: &Rtti) -> Option<StructInfo> {
        // Implementation based on RTTI structure
        todo!()
    }

    // NEW: Enum variant information

    /// Get all variants for an enum type.
    pub fn enum_variants(&self, hash: Hash) -> Option<Vec<EnumVariantInfo>> {
        todo!()
    }
}

// crates/rune/src/compile/type_info.rs

/// Information about a struct type.
#[derive(Debug, Clone)]
pub struct StructInfo {
    /// Struct name
    pub name: String,
    /// Full path
    pub path: String,
    /// Type hash
    pub hash: Hash,
    /// Fields with optional type annotations
    pub fields: Vec<FieldInfo>,
}

/// Information about a struct field.
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field name
    pub name: String,
    /// Field position (for positional access)
    pub position: usize,
    /// Type annotation (if present)
    pub type_info: Option<AnnotatedType>,
}

/// Information about an enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    /// Variant name
    pub name: String,
    /// Variant hash
    pub hash: Hash,
    /// Fields (for struct variants)
    pub fields: Vec<FieldInfo>,
}
```

---

#### T3.2: Improve Error Messages

**Priority**: Low
**Complexity**: Low
**Files**: `typeck.rs`
**Parallel**: Yes

**Current** (typeck.rs:625-630):
```rust
if options.strict_types {
    return Err(compile::Error::msg(
        span.span(),
        format!("Type mismatch: expected `{expected_str}`, found `{actual_str}`"),
    ));
}
```

**Improved**:
```rust
// crates/rune/src/compile/error.rs

/// Add new ErrorKind variant
pub(crate) enum ErrorKind {
    // ... existing ...

    /// Type mismatch in gradual typing
    TypeMismatch {
        /// Expected type
        expected: Box<str>,
        /// Actual type found
        actual: Box<str>,
        /// Context (return, argument, field, etc.)
        context: TypeContext,
    },
}

/// Context for type mismatch errors
#[derive(Debug, Clone, Copy)]
pub enum TypeContext {
    Return,
    Argument { position: usize, name: Option<Box<str>> },
    StructField { field: Box<str> },
    Assignment,
}

// Usage in typeck.rs:
return Err(compile::Error::new(
    span.span(),
    ErrorKind::TypeMismatch {
        expected: expected_str.into(),
        actual: actual_str.into(),
        context: TypeContext::Return,
    },
));
```

**Better Display**:
```rust
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::TypeMismatch { expected, actual, context } => {
                write!(f, "type mismatch: expected `{}`, found `{}`", expected, actual)?;
                match context {
                    TypeContext::Return => write!(f, " in return expression"),
                    TypeContext::Argument { position, name: Some(n) } => {
                        write!(f, " for argument `{}` (position {})", n, position)
                    }
                    TypeContext::Argument { position, name: None } => {
                        write!(f, " for argument at position {}", position)
                    }
                    TypeContext::StructField { field } => {
                        write!(f, " for field `{}`", field)
                    }
                    TypeContext::Assignment => write!(f, " in assignment"),
                }
            }
            // ...
        }
    }
}
```

---

#### T3.3: Add Call-Site Argument Checking (Optional/Future)

**Priority**: Low (future enhancement)
**Complexity**: High
**Files**: `typeck.rs`, `lowering.rs`

**Current Limitation**: Only return types are checked, not arguments at call sites.

**Design Considerations**:
1. Need to track function signatures in type checker
2. Must resolve called function to get expected parameter types
3. Handle closures, methods, and indirect calls differently
4. Performance impact of signature lookup

**Skeleton Implementation**:
```rust
// crates/rune/src/hir/typeck.rs

impl TypeChecker<'_> {
    /// Check a function call expression against known signatures.
    ///
    /// This is an optional enhancement that validates argument types
    /// at call sites when the callee's signature is known.
    pub(crate) fn check_call(
        &mut self,
        q: &mut Query<'_, '_>,
        source_id: SourceId,
        call: &ast::ExprCall,
        options: &Options,
    ) -> compile::Result<()> {
        // 1. Resolve the callee to get its hash
        let callee_hash = match &call.expr {
            ast::Expr::Path(path) => {
                let named = q.convert_path(path)?;
                q.query_meta(path, named.item, Default::default())?
                    .map(|m| m.hash)
            }
            _ => None, // Indirect calls - skip for now
        };

        let Some(hash) = callee_hash else {
            return Ok(()); // Can't determine callee, skip
        };

        // 2. Look up the callee's signature
        let Some(sig) = q.lookup_function_signature(hash) else {
            return Ok(()); // No signature info available
        };

        // 3. Check each argument
        for (i, (arg_expr, _)) in call.args.iter().enumerate() {
            if let Some(expected_type) = sig.param_type(i) {
                let actual_type = self.infer_expr(q.sources, source_id, arg_expr)?;
                let actual_resolved = self.apply(&actual_type)?;

                if !actual_resolved.is_compatible_with(&expected_type) {
                    self.emit_type_mismatch(
                        q,
                        source_id,
                        arg_expr,
                        &expected_type,
                        &actual_resolved,
                        options,
                    )?;
                }
            }
        }

        Ok(())
    }
}
```

---

### P4: Cleanup Tasks

#### T4.1: Squash/Rebase Commits

**Priority**: Medium (before PR submission)
**Complexity**: Low

**Current History Issues**:
- Duplicate commits for parsing
- Feature flag added then removed
- Multiple integration commits

**Proposed Clean History**:
```
1. feat: add type annotation parsing
   - FnArg::Typed variant
   - Parsing for `name: Type` syntax
   - Tests for parsing

2. feat(gradual-typing): add type checking infrastructure
   - ResolvedType enum
   - TypeChecker struct
   - Builtin type hash caching
   - String interning

3. feat(gradual-typing): integrate type checking into HIR lowering
   - TypeChecker in Ctxt
   - Function parameter/return type checking
   - Struct field type checking

4. feat(gradual-typing): add protocol lookups for operators
   - Protocol return type lookup
   - Arithmetic/comparison operator handling

5. feat(gradual-typing): add type extraction API
   - FunctionSignature API
   - AnnotatedType parsing
   - Unit methods for extraction

6. test(gradual-typing): comprehensive test suite
   - All test categories

7. docs(gradual-typing): add example and documentation
   - type_extraction example
   - Module documentation
```

**Commands**:
```bash
# From feature branch
git rebase -i main

# In editor, squash related commits
# Mark as 'squash' or 's' to combine

# Force push if branch already pushed
git push --force-with-lease origin feature/gradual-typing
```

---

#### T4.2: Update Examples

**Priority**: Low
**Files**: `examples/examples/type_extraction.rs`
**Parallel**: Yes

**Current Example** is good but could show more features:

```rust
// examples/examples/type_extraction.rs

//! Demonstration of Rune's type extraction API for gradual typing.
//!
//! This example shows how to:
//! - Extract function signatures from compiled units
//! - Access parameter and return type information
//! - Query structs and their field types (when available)

use rune::{Context, Diagnostics, Source, Sources, Vm};
use std::sync::Arc;

fn main() -> rune::support::Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = Sources::new();
    sources.insert(Source::new("example", r#"
        /// A 2D point with typed coordinates.
        struct Point {
            x: i64,
            y: i64,
        }

        /// Add two numbers together.
        fn add(a: i64, b: i64) -> i64 {
            a + b
        }

        /// Create a point at the origin.
        fn origin() -> Point {
            Point { x: 0, y: 0 }
        }

        /// Mixed typing: typed return, untyped parameter.
        fn double(n) -> i64 {
            n * 2
        }

        /// Untyped function (gradual typing compatible).
        fn untyped_add(a, b) {
            a + b
        }
    "#)?);

    let mut diagnostics = Diagnostics::new();
    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;
    let unit = Arc::new(unit);

    println!("=== Function Signatures ===\n");

    for sig in unit.function_signatures() {
        println!("Function: {}", sig.path);
        println!("  Hash: {:?}", sig.hash);
        println!("  Async: {}", sig.is_async);

        if sig.parameters.is_empty() {
            println!("  Parameters: none");
        } else {
            println!("  Parameters:");
            for param in &sig.parameters {
                match &param.type_info {
                    Some(ty) => println!("    {}: {:?}", param.name, ty),
                    None => println!("    {}: (untyped)", param.name),
                }
            }
        }

        match &sig.return_type {
            Some(ty) => println!("  Returns: {:?}", ty),
            None => println!("  Returns: (untyped)"),
        }

        println!();
    }

    // Query specific function
    println!("=== Query by Name ===\n");
    if let Some(sig) = unit.function_signature_by_name("add") {
        println!("Found 'add': {} parameters, returns {:?}",
                 sig.parameters.len(),
                 sig.return_type);
    }

    Ok(())
}
```

---

#### T4.3: Final Documentation

**Priority**: Low (before merge)
**Files**: Various

**Checklist**:
- [ ] Module-level docs in `typeck.rs`
- [ ] Public API docs in `unit.rs` (type extraction methods)
- [ ] Update CHANGELOG.md (if exists)
- [ ] Ensure all public types have `///` docs
- [ ] Add `#[doc(hidden)]` to internal helpers if needed

---

## Part 4: Implementation Schedule

### Parallel Execution Groups

**Group A (Independent Foundation)**:
- T1.1: Consolidate lowering paths
- T1.2: Add closure type checking
- T1.3: Fix clippy warnings

**Group B (Test Consolidation)**:
- T2.1: Consolidate test files
- T2.2: Add missing coverage (after T1.2)
- T2.3: Add documentation

**Group C (API Work)**:
- T3.1: Extend type extraction API
- T3.2: Improve error messages

**Group D (Final Cleanup)**:
- T4.1: Squash/rebase commits
- T4.2: Update examples
- T4.3: Final documentation

### Suggested Execution Order

```
Phase 1 (Parallel):
  Agent 1: T1.1 (lowering consolidation)
  Agent 2: T1.2 (closure type checking)
  Agent 3: T1.3 (clippy fixes)

Phase 2 (Parallel):
  Agent 1: T2.1 (test consolidation)
  Agent 2: T3.1 (API extension)
  Agent 3: T3.2 (error messages)

Phase 3 (Sequential):
  T2.2 (missing coverage - needs T1.2, T2.1)
  T2.3 (documentation)

Phase 4 (Final):
  T4.1, T4.2, T4.3 (cleanup and polish)
```

---

## Appendix A: Code Skeletons

### A.1: TypeChecker for lowering2.rs

```rust
// Add to crates/rune/src/hir/lowering2.rs top imports:
use crate::hir::typeck::{ResolvedType, TypeChecker};

// Helper function to extract return type from stream
fn parse_return_type<'a>(
    cx: &mut Ctxt<'_, '_, '_>,
    p: &mut Stream<'a>,
) -> Result<Option<ast::Type>> {
    // Look for -> Type syntax in stream
    if p.eat(K![->]).is_some() {
        let ty = p.expect(Type)?.parse(|p| {
            // Parse type from stream
            Ok(todo!("parse type AST"))
        })?;
        Ok(Some(ty))
    } else {
        Ok(None)
    }
}

// In item_fn, after parsing args:
pub(crate) fn item_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    is_instance: bool,
) -> Result<hir::ItemFn<'hir>> {
    // ... existing parsing ...

    // NEW: Check for return type annotation
    let return_type = parse_return_type(cx, p)?;

    if let Some(ref ret_ty) = return_type {
        let expected = ResolvedType::from_ast_type(ret_ty, &mut cx.q, cx.source_id)?;
        let typeck = TypeChecker::new(cx.q.context, Some(expected))?;
        cx.typeck = Some(typeck);
    }

    let body = p.expect(Block)?.parse(|p| block(cx, None, p))?;

    // NEW: Finalize type checking
    if let Some(ref mut typeck) = cx.typeck {
        let options = cx.q.options;
        typeck.finalize(&mut cx.q, cx.source_id, &p, options)?;
    }
    cx.typeck = None;

    Ok(hir::ItemFn { ... })
}
```

### A.2: Test Consolidation Module Structure

```rust
// crates/rune/src/tests/gradual_typing/mod.rs
//! Gradual typing test suite.

prelude!();

mod basic;
mod diagnostics;
mod extraction;
mod inference;
mod protocols;

// Shared test utilities
pub(super) fn compile_with_options(
    source: &str,
    strict: bool,
) -> (Unit, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let mut options = Options::default();
    options.strict_types = strict;

    let unit = crate::tests::compile_helper(source, &mut diagnostics)
        .expect("should compile");

    (unit, diagnostics)
}
```

### A.3: StructInfo Implementation

```rust
// crates/rune/src/runtime/unit.rs

impl<S> Unit<S> {
    fn build_struct_info(&self, rtti: &Arc<Rtti>) -> Option<StructInfo> {
        use crate::alloc::prelude::*;
        use crate::compile::type_info::{FieldInfo, StructInfo};

        let name = rtti.item
            .last()
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .try_to_string()
            .ok()?;

        let path = rtti.item.try_to_string().ok()?;

        let fields = rtti.fields
            .iter()
            .enumerate()
            .map(|(position, field_name)| {
                FieldInfo {
                    name: field_name.try_to_string().ok()?,
                    position,
                    type_info: None, // Field types not in RTTI currently
                }
            })
            .collect::<Option<Vec<_>>>()?;

        Some(StructInfo {
            name,
            path,
            hash: rtti.hash,
            fields,
        })
    }
}
```

---

## Appendix B: Test Case Templates

### B.1: Basic Type Checking Tests

```rust
// crates/rune/src/tests/gradual_typing/basic.rs

prelude!();

/// Functions with return type annotations should compile and execute
#[test]
fn return_type_annotation_works() {
    let result: i64 = rune! {
        fn add(a, b) -> i64 {
            a + b
        }
        add(1, 2)
    };
    assert_eq!(result, 3);
}

/// Parameter type annotations should compile and execute
#[test]
fn param_type_annotations_work() {
    let result: i64 = rune! {
        fn add(a: i64, b: i64) -> i64 {
            a + b
        }
        add(1, 2)
    };
    assert_eq!(result, 3);
}

/// Full signature with all types should work
#[test]
fn full_signature_works() {
    let result: String = rune! {
        fn greet(name: String) -> String {
            `Hello, {name}!`
        }
        greet("World")
    };
    assert_eq!(result, "Hello, World!");
}
```

### B.2: Diagnostics Tests

```rust
// crates/rune/src/tests/gradual_typing/diagnostics.rs

prelude!();
use crate::diagnostics::WarningDiagnosticKind;

/// Type mismatch in return should produce warning (non-strict mode)
#[test]
fn warn_return_type_mismatch() {
    assert_warnings! {
        r#"
        fn foo() -> i64 {
            "not an i64"
        }
        foo()
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Type mismatch should be error in strict mode
#[test]
fn error_in_strict_mode() {
    let mut diagnostics = Diagnostics::new();
    let mut options = Options::default();
    options.strict_types = true;

    let result = rune::prepare(&mut Sources::new())
        .with_source(Source::new("test", r#"
            fn foo() -> i64 { "wrong" }
        "#))
        .with_options(&options)
        .with_diagnostics(&mut diagnostics)
        .build();

    assert!(result.is_err() || diagnostics.has_error());
}
```

---

## Summary

This plan provides a comprehensive roadmap for refreshing and improving the gradual typing implementation. Key priorities:

1. **Fix the dual lowering path issue** (T1.1) - Critical for correctness
2. **Add closure type checking** (T1.2) - Important for completeness
3. **Consolidate test files** (T2.1) - Improves maintainability
4. **Squash commits** (T4.1) - Cleaner PR for review

The tasks are organized for parallel execution where possible, with clear dependencies and detailed implementation guidance.
