# Squash/Rebase Plan for Gradual Typing PR

## Current State
- 32 commits since main (after adding closure return types)
- Mix of features, fixes, refactors, and tests
- Some duplicated work (e.g., two "integrate type checking into HIR lowering" commits)

## Proposed Logical Commits (6 total)

### Commit 1: feat(ast): add type annotation syntax support
**Files**: `ast/fn_arg.rs`, `ast/expr_closure.rs`
**Content**:
- FnArg::Typed variant for `name: Type` syntax in function/closure parameters
- Closure return type syntax: `|x: i64| -> i64 { x }`
- Parser tests for all type annotation scenarios

**Original commits**:
```
0dee73b1 feat: add type annotation parsing (not yet supported)
e64cb38d feat: add type annotation parsing (not yet supported)
5d62e42f feat(gradual-typing): add closure return type annotation syntax
```

### Commit 2: feat(typeck): add type checking and inference infrastructure
**Files**: `hir/typeck.rs`
**Content**:
- ResolvedType enum (Named, Tuple, Any, Never, Variable)
- TypeVar for inference
- TypeChecker struct with unification algorithm
- BUILTIN_TYPE_HASHES cache
- Type variable scoping and management

**Original commits**:
```
b1b0b2e8 feat(gradual-typing): add type checking and inference infrastructure
fbadf890 refactor(gradual-typing): remove duplicate compile/typeck.rs
4e34b837 fix(gradual-typing): improve safety and documentation in type checker
65a137b9 refactor(gradual-typing): improve API ergonomics and efficiency
b8817d34 perf(gradual-typing): use Vec for scope storage instead of HashMap
0d737564 refactor(typeck): optimize type checking performance and architecture
```

### Commit 3: feat(hir): integrate type checking into HIR lowering
**Files**: `hir/lowering.rs`, `hir/ctxt.rs`, `indexing/index.rs`
**Content**:
- TypeChecker integration into function/closure lowering
- Function return type checking
- Struct field type checking
- Closure parameter and return type checking
- Type annotation capture during indexing

**Original commits**:
```
a0d9d699 feat(gradual-typing): capture type annotations during indexing
75b40313 feat(gradual-typing): integrate type checking into compilation pipeline
642567fb feat(gradual-typing): integrate type checking into HIR lowering
183348cc feat(gradual-typing): implement struct field type checking
1caa7d15 feat(gradual-typing): integrate type checking into HIR lowering
```

### Commit 4: feat(typeck): add protocol lookups for operator return types
**Files**: `hir/typeck.rs` (protocol sections)
**Content**:
- Protocol lookups for binary/unary operators
- binop_to_protocol() mapping
- lookup_protocol_return_type()
- Builtin arithmetic type special handling

**Original commits**:
```
6833c92b feat(gradual-typing): add protocol lookups for operator return types
```

### Commit 5: feat(runtime): add type extraction API
**Files**: `compile/type_info.rs`, `runtime/unit.rs`, `runtime/debug.rs`, `examples/`
**Content**:
- AnnotatedType, ParameterType, FunctionSignature structs
- StructInfo, FieldInfo for struct metadata
- Unit::function_signatures(), struct_infos()
- Lookup methods by name and hash
- type_extraction example

**Original commits**:
```
fee00ff8 feat(gradual-typing): add type extraction API to runtime Unit
7555a358 feat(gradual-typing): un-gate type signature information
ea1f0b39 docs(examples): add type_extraction example for gradual typing API
7eac105c feat(gradual-typing): extend type extraction API and fix test suite
```

### Commit 6: test(gradual-typing): add comprehensive test suite and options
**Files**: `tests/gradual_typing*.rs`, `compile/options.rs`
**Content**:
- Comprehensive test suite (main, edge cases, errors, inference, performance, protocols)
- strict_types compile option
- TypeMismatch warning diagnostic
- Feature flag removal (always enabled)
- Various bug fixes and cleanups

**Original commits**:
```
69292c7c feat(gradual-typing): add strict_types option and TypeMismatch warning
23f80606 feat(gradual-typing): add gradual-typing feature flag to Cargo.toml
91516a8d test(gradual-typing): add comprehensive test suite
885540c2 test(gradual-typing): add comprehensive test suite
d88aad58 fix: resolve clippy only-used-in-recursion violation
ee0fd90f refactor: clean up imports and optimize closures
3817a0ce refactor: remove gradual-typing feature flag
ad8bffdd fix(gradual-typing): address QA review issues
e6e01b4d refactor(tests): balance test coverage - focus on realistic + extreme cases
2c8f4b2d chore: editor syntax highlighting and code cleanup
57e34e0e test(gradual-typing): add protocol lookup tests
25d6a033 chore: update remaining DocType → TypeHash renames
467a3d2c test(gradual-typing): add stdlib type annotation tests
fcee57c0 refactor(gradual-typing): use is_some_and for cleaner code
```

## Execution Steps

### Step 1: Create backup
```bash
git branch backup-before-squash
```

### Step 2: Interactive rebase
```bash
git rebase -i main
```

### Step 3: In editor, reorder and mark commits

The editor will show commits oldest-first. Reorder them by logical group and mark squashes:

```
# Commit 1: Parser/AST
pick 0dee73b1 feat: add type annotation parsing (not yet supported)
squash e64cb38d feat: add type annotation parsing (not yet supported)
squash 5d62e42f feat(gradual-typing): add closure return type annotation syntax

# Commit 2: Type checking infrastructure
pick b1b0b2e8 feat(gradual-typing): add type checking and inference infrastructure
squash fbadf890 refactor(gradual-typing): remove duplicate compile/typeck.rs
squash 4e34b837 fix(gradual-typing): improve safety and documentation in type checker
squash 65a137b9 refactor(gradual-typing): improve API ergonomics and efficiency
squash b8817d34 perf(gradual-typing): use Vec for scope storage instead of HashMap
squash 0d737564 refactor(typeck): optimize type checking performance and architecture

# Commit 3: HIR lowering integration
pick a0d9d699 feat(gradual-typing): capture type annotations during indexing
squash 75b40313 feat(gradual-typing): integrate type checking into compilation pipeline
squash 642567fb feat(gradual-typing): integrate type checking into HIR lowering
squash 183348cc feat(gradual-typing): implement struct field type checking
squash 1caa7d15 feat(gradual-typing): integrate type checking into HIR lowering

# Commit 4: Protocol lookups
pick 6833c92b feat(gradual-typing): add protocol lookups for operator return types

# Commit 5: Type extraction API
pick fee00ff8 feat(gradual-typing): add type extraction API to runtime Unit
squash 7555a358 feat(gradual-typing): un-gate type signature information
squash ea1f0b39 docs(examples): add type_extraction example for gradual typing API
squash 7eac105c feat(gradual-typing): extend type extraction API and fix test suite

# Commit 6: Tests and options
pick 69292c7c feat(gradual-typing): add strict_types option and TypeMismatch warning
squash 23f80606 feat(gradual-typing): add gradual-typing feature flag to Cargo.toml
squash 91516a8d test(gradual-typing): add comprehensive test suite
squash 885540c2 test(gradual-typing): add comprehensive test suite
squash d88aad58 fix: resolve clippy only-used-in-recursion violation
squash ee0fd90f refactor: clean up imports and optimize closures
squash 3817a0ce refactor: remove gradual-typing feature flag
squash ad8bffdd fix(gradual-typing): address QA review issues
squash e6e01b4d refactor(tests): balance test coverage - focus on realistic + extreme cases
squash 2c8f4b2d chore: editor syntax highlighting and code cleanup
squash 57e34e0e test(gradual-typing): add protocol lookup tests
squash 25d6a033 chore: update remaining DocType → TypeHash renames
squash 467a3d2c test(gradual-typing): add stdlib type annotation tests
squash fcee57c0 refactor(gradual-typing): use is_some_and for cleaner code
```

### Step 4: Write commit messages

For each squashed group, write a clear commit message:

**Commit 1:**
```
feat(ast): add type annotation syntax support

Add support for type annotations in function parameters and closure return types:

- FnArg::Typed variant for `name: Type` syntax in fn/closure parameters
- Closure return type syntax: `|x: i64| -> i64 { x }`
- Parser tests for all type annotation scenarios

This enables the gradual typing syntax that allows optional type annotations
while maintaining backwards compatibility with untyped code.
```

**Commit 2:**
```
feat(typeck): add type checking and inference infrastructure

Add the core type checking and inference system:

- ResolvedType enum: Named, Tuple, Any, Never, Variable
- TypeVar for type inference with union-find structure
- TypeChecker struct with unification algorithm
- BUILTIN_TYPE_HASHES lazy cache for efficient type resolution
- Scope-based type variable management using Vec for performance

The type checker implements gradual typing semantics where Any is
compatible with all types, enabling incremental adoption of type
annotations.
```

**Commit 3:**
```
feat(hir): integrate type checking into HIR lowering

Integrate the type checker into the HIR lowering phase:

- TypeChecker initialization in function/closure lowering
- Function return type checking with TypeMismatch warnings
- Struct field type checking during struct literal lowering
- Closure parameter and return type checking
- Type annotation capture during indexing phase

Type mismatches produce warnings (not errors) to maintain gradual typing
semantics and allow incremental adoption.
```

**Commit 4:**
```
feat(typeck): add protocol lookups for operator return types

Add type inference for binary and unary operators through protocol lookups:

- binop_to_protocol() mapping for binary operators
- lookup_protocol_return_type() for protocol-based type resolution
- Special handling for builtin arithmetic types (i64, f64, etc.)

This enables type inference to flow through arithmetic operations:
`let x: i64 = a + b;` can infer that a and b should be i64.
```

**Commit 5:**
```
feat(runtime): add type extraction API

Add API for extracting type information from compiled units:

- AnnotatedType, ParameterType, FunctionSignature structs
- StructInfo, FieldInfo for struct metadata
- Unit::function_signatures() iterator
- Unit::function_signature(hash) and function_signature_by_name(name)
- Unit::struct_infos() iterator with lookup methods
- type_extraction example demonstrating the API

This enables IDE integration and documentation generation by exposing
type annotations from compiled Rune code.
```

**Commit 6:**
```
test(gradual-typing): add comprehensive test suite and options

Add test suite and compile options for gradual typing:

- gradual_typing.rs: Core type annotation tests
- gradual_typing_edge_cases.rs: Boundary condition tests
- gradual_typing_errors.rs: Type mismatch warning tests
- gradual_typing_inference.rs: Type inference tests
- gradual_typing_performance.rs: Performance regression tests
- gradual_typing_protocols.rs: Protocol type inference tests
- strict_types compile option for stricter checking
- TypeMismatch warning diagnostic
- Various bug fixes and code cleanup
```

### Step 5: Verify and push
```bash
# Run tests to verify
cargo test -p rune --features workspace

# Force push (if branch was already pushed)
git push --force-with-lease
```

## Alternative: More Granular (8 commits)

If 6 is too coarse, consider splitting:
- Commit 3 → 3a (function types) + 3b (struct types)
- Commit 6 → 6a (tests) + 6b (options/cleanup)

## Notes

- Interactive rebase (`git rebase -i`) requires manual input and cannot be automated
- Each squashed commit group will prompt for a new commit message
- Use `git rebase --abort` if something goes wrong
- The backup branch allows recovery: `git reset --hard backup-before-squash`
