# Compiler guide

This is intended to be a guide into the compiler architecture for Rune for
people who want to hack on it.

> **Rune is in heavy development** and this section is likely to change a lot.

Compiling a rune program involves the following stages:

* Queue the initial source files specified by [`Source::insert`].
* **Indexing and macro expansion**, which processes tasks in the [`Worker`]
  queue until it is empty. These are:
  * `Task::LoadFile ` - Loads a single source into [`AST`] file and indexes it.
  * `Task::ExpandUnitWildcard` - A deferred expansion of a wildcard import. This
    must happen after indexing because macros might expand into imports.
* **Compilation** which processes a queue of items to be compiled and assembled.

## Indexing

Indexing is primarily handled through the [`Index`] trait, which are
implemented for the type being indexed with the helper of the [`Indexer`].

This walks through the [`AST`] to be indexed and construct [components] into an
item path for every:
* Functions, which adds components named after the function. `fn foo` would add
  `foo`.
* Closures, blocks, and nested functions, which adds an id component, like `$10`
  where the number depends on how many sibling components there are. These are
  effectively anonymous, and can't be referenced through the language directly.

## Compilation

The compilation stage processed the entire [`AST`] of every function that is
queued to be compiled and generates a sequence of instructions for them through
implementations of the [`Assemble`] trait.

This stage uses the [`Query`] system to look up metadata about external items,
and any external item queried for is subsequently queued up to be built.

Consider the following unit:

```rune
{{#include ../../scripts/book/compiler_guide/dead_code.rn}}
```

Let's dump all dynamic functions in it:

```text
$> cargo run --bin rune -- run scripts/book/compiler_guide/dead_code.rn --dump-functions
# dynamic functions
0xe7fc1d6083100dcd = main()
0x20c6d8dd92b51018 = main::$0::foo()
---
== 2 (59.8µs)
```

As you can see, the code for `main::$0::bar` was *never generated*. This is
because it's a local function that is never called. And therefore never queried
for. So it's never queued to be built in the compilation stage.

## State during compilation

Each item in the AST is relatively isolated while they are being compiled. This
is one of the benefits of compiling for a stack-based virtual machine - the
compilation stage is relatively simple and *most* reasoning about what
instructions to emit can be made locally.

> Note that this quickly changes if you want to perform most forms of
> optimizations. But it's definitely true for naive (and therefore fast!) code
> generation.

While compiling we keep track of the following state in the [`Compiler`]

The source file and id that we are compiling for and global storage used for
macro-generated identifiers and literals. This is used to resolve values from
the AST through the corresponding [`Resolve`] implementation. An example of this
is the [`Resolve` implementation of `LitStr`].

We keep track of local variables using [`Scopes`]. Each block creates a new
scope of local variables, and this is simply a number that is incremented each
time a variable is allocated. These can either be named or anonymous. Each named
variable is associated with an offset relative to the current [call
frame](./call_frames.md) that can be looked up when a variable needs to be used.

We maintain information on loops we're through [`Loops`]. This is a stack that
contains every loop we are nested in, information on the label in which the loop
terminates, and locals that would have to be cleaned up in case we encounter a
[`break` expression].

There are a couple more traits which are interesting during compilation:
* `AssembleConst` - used for assembling constants.
* `AssembleFn` - used for assembling the content of functions.
* `AssembleClosure` - used for assembling closures.

Let's look closer at how closures are assembled through AssembleClosure. Once a
closure is queried for, it is queued up to be built by the query system. The
closure procedure would be compiled and inserted into the unit separately at a
given item (like `main::$0::$0`). And when we invoke the closure, we assemble a
*call* to this procedure.

We can see this call by dumping all the dynamic functions in the following
script:

```rune
{{#include ../../scripts/book/compiler_guide/closures.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/compiler_guide/closures.rn --emit-instructions --dump-functions
# instructions
fn main() (0x1c69d5964e831fc1):
  0000 = load-fn hash=0xbef6d5f6276cd45e // closure `3`
  0001 = copy offset=0 // var `callable`
  0002 = call-fn args=0
  0003 = pop
  0004 = pop
  0005 = return-unit

fn main::$0::$0() (0xbef6d5f6276cd45e):
  0006 = push value=42
  0007 = return address=top, clean=0
# dynamic functions
0xbef6d5f6276cd45e = main::$0::$0()
0x1c69d5964e831fc1 = main()
```

A function pointer is pushed on the stack `load-fn 0xca35663d3c51a903`, then
copied and called with zero arguments.

[`Assemble`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/assemble/mod.rs
[`AST`]: https://github.com/rune-rs/rune/tree/main/crates/rune/src/ast
[`break` expression]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/assemble/expr_break.rs
[`closure` expression]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/assemble/expr_closure.rs
[`Compiler`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/compiler.rs
[`Index`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/indexing/index.rs
[`Indexer`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/indexing/index.rs
[`Items`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/shared/items.rs
[`Loops`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/loops.rs
[`Query`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/query.rs
[`Resolve` implementation of `LitStr`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/ast/lit_str.rs
[`Resolve`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/parsing/resolve.rs
[`Scopes`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/compiling/scopes.rs
[`Source::insert`]: https://docs.rs/rune/0/rune/struct.Source.html#method.insert
[`Worker`]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/worker.rs
[components]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/item.rs
