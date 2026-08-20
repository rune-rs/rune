# Statics

A `static` item is a named piece of mutable state that lives for as long as the
program using it, rather than for the duration of a single call.

```rune
{{#include ../../scripts/book/statics/counter.rn}}
```

```text
$> cargo run -- run scripts/book/statics/counter.rn
1
2
3
```

This is the main thing that distinguishes a `static` from a [`const`]. A
constant is *inlined* at every place it is used, so each use produces a fresh
copy and there is nothing to assign to. A static instead occupies a numbered
slot, and every use reads or writes that one slot.

Statics are declared at the item level, so they can appear inside modules and be
addressed by path like any other item:

```rune
pub mod config {
    pub static WIDTH = 800;
}

println!("{}", config::WIDTH);
```

## Initializers

The expression a static is initialized with must be a constant expression, the
same kind that a `const` accepts. It is evaluated lazily, the first time
something reads the static:

```rune
static LIMIT = 4 * 8;
```

The initializer is optional. A static declared without one starts out
uninitialized, and reading it before anything has assigned it is an error:

```rune
{{#include ../../scripts/book/statics/uninitialized.rn}}
```

```text
$> cargo run -- run scripts/book/statics/uninitialized.rn
error: Reading uninitialized static `CONFIG` (slot 0)
  ┌─ scripts/book/statics/uninitialized.rn:3:10
  │
3 │ println!("{CONFIG}");
  │          ^^^^^^^^^^ Reading uninitialized static `CONFIG` (slot 0)

Backtrace:
scripts/book/statics/uninitialized.rn:3:10:
println!("{CONFIG}");
```

> Note that the static is named in the diagnostic. This relies on debug
> information being available. Without it the message falls back to reporting
> the slot number alone.

Since a static isn't known until runtime, it cannot be used anywhere a value is
required at compile time. Referring to one from a `const`, from another
static's initializer, or from a pattern is a compile error.

## Statics hold values, not copies

[As with every other value in Rune](./variables.md), what a static holds is a
reference counted value. Reading a static therefore hands you a handle to *the
same* instance, so mutating through that handle mutates what the static holds
without any assignment taking place:

```rune
{{#include ../../scripts/book/statics/shared_value.rn}}
```

```text
$> cargo run -- run scripts/book/statics/shared_value.rn
[1, 2]
```

## Storage is configured on the virtual machine

The values of statics are *not* stored in the [`Unit`]. A unit only records how
many statics there are, which slot each one was assigned, and what each one's
initializer is. The values live in a separate [`Globals`] storage which is
handed to the virtual machine when it is constructed:

```rust
let globals = Globals::new(unit.clone())?;
let mut vm = Vm::new(runtime, unit).with_globals(globals);
```

This separation means one compiled unit can back any number of independent
states - one per tenant, per session, per test - simply by constructing more
than one `Globals` for it. Two machines built from the same unit with different
storage do not observe each other's statics.

[`Globals`] is a cheaply cloned handle, so the caller can hold on to a clone and
read or write statics while the machine that uses them is running. Statics are
addressed by item, the same way functions are:

```rust
let globals = Globals::new(unit.clone())?;

// Assign before anything runs.
globals.set(["COUNTER"], rune::to_value(41i64)?)?;

let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
vm.call(["main"], ())?;

// Read back what the script left behind.
let value = globals.get(["COUNTER"])?;
```

Writing a static from the outside takes precedence over its initializer, since
the initializer only runs for a slot which is still uninitialized.

If you construct a `Vm` without calling [`Vm::with_globals`], it has no storage
and reading a static errors. [`Build::build_vm`] provisions storage for you, so
the shorthand path needs no extra setup:

```rust
let mut vm = rune::prepare(&mut sources)
    .with_context(&context)
    .build_vm()?;
```

## Providing a global registry

The most useful thing this enables is handing scripts access to something the
host owns - a registry, a service handle, a configuration object - without
threading it through the signature of every function that might need it.

Declare a static with no initializer and let the host fill it in:

```rune
static REGISTRY;

fn describe(key) {
    match REGISTRY.get(key) {
        Some(value) => format!("{key} is {value}"),
        None => format!("{key} is unset"),
    }
}
```

`describe` takes no registry argument, and neither does anything that calls it.
The host writes the slot once, before the first call:

```rust
{{#include ../../examples/examples/global_registry.rs}}
```

```text
$> cargo run --example global_registry
width is 800
height is 600
depth is unset
```

Because the storage is per-machine rather than per-unit, this stays sound when
the same script is used for many independent things at once - each caller
constructs its own `Globals` and injects its own registry.

## Declaring statics with the build

A host which owns the state doesn't necessarily want the script to be
responsible for declaring it. Statics can therefore also be declared with the
build through [`Statics`], in which case the source doesn't mention them at all:

```rust
let mut statics = Statics::new();
statics.insert(["REGISTRY"])?;

let unit = rune::prepare(&mut sources)
    .with_context(&context)
    .with_statics(&statics)
    .build()?;
```

The script above then works unchanged with its `static REGISTRY;` removed, in
the same way that a native module makes a function available without the script
declaring it. Every source in the unit sees the static, and the name is an item,
so `["config", "REGISTRY"]` declares it inside of the `config` module.

There is no source to evaluate, so a static declared this way has no
initializer. It starts out uninitialized just like `static REGISTRY;` does, and
the caller is expected to assign it before anything reads it:

```rust
{{#include ../../examples/examples/declared_statics.rs}}
```

```text
$> cargo run --example declared_statics
Hello, Jane! (call 1)
Hello, John! (call 2)
calls: 2
```

Since a declared static occupies the item it is named with, a source which
declares an item of the same name is a compile error rather than a silent
override of either one.

## Statics and threads

Static storage holds [`Value`]'s, so like the [`Vm`] itself it cannot be shared
across threads. Each thread builds its own [`Globals`] out of the shared
[`Unit`]; see [Multithreading] for the general picture.

That does not stop statics from being shared state across threads, it just moves
the responsibility. Assign a host value whose interior is synchronized, such as
an `Arc<Mutex<..>>`, an atomic, or a channel sender, and every thread's slot
points at the same interior, with that value deciding how concurrent access is
handled.
This pairs naturally with a pool of machines: a pooled machine keeps the storage
it was built with, so a worker that takes one out of the pool already has its
statics wired up. See [Statics across threads] for a worked example.

One consequence is worth calling out: [`Function::into_sync`] produces a
[`SyncFunction`], which cannot carry the storage. Calling a static through a
`SyncFunction` reports that no storage has been configured.

[`Build::build_vm`]: https://docs.rs/rune/latest/rune/struct.Build.html#method.build_vm
[`const`]: ./items_imports.md
[`Function::into_sync`]: https://docs.rs/rune/latest/rune/runtime/struct.Function.html#method.into_sync
[`Globals`]: https://docs.rs/rune/latest/rune/runtime/struct.Globals.html
[`Statics`]: https://docs.rs/rune/latest/rune/struct.Statics.html
[`SyncFunction`]: https://docs.rs/rune/latest/rune/runtime/struct.SyncFunction.html
[`Unit`]: https://docs.rs/rune/latest/rune/struct.Unit.html
[`Value`]: https://docs.rs/rune/latest/rune/runtime/struct.Value.html
[`Vm::with_globals`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html#method.with_globals
[`Vm`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html
[Multithreading]: ./multithreading.md
[Statics across threads]: ./multithreading.md#statics-across-threads
