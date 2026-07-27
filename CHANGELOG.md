# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to
[Semantic Versioning].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

## [Unreleased] - 0.15.0

### Added

#### `static` items

Scripts can declare `static` items, which are named pieces of mutable state that
live for as long as the program using them rather than for a single call. See
the [Statics] chapter of the book.

```rune
static COUNTER = 0;

fn bump() {
    COUNTER += 1;
    COUNTER
}
```

Where a `const` is inlined at every place it is used, a `static` occupies a
numbered slot which every use reads or writes. Scripts can assign to a static
with `=` and with the compound operators such as `+=`.

An initializer is optional and has to be a constant expression. It is evaluated
lazily, the first time something reads the slot. A static declared without one,
such as `static CONFIG;`, starts out uninitialized, and reading it before
anything has assigned it is a runtime error.

Since a static is not known until runtime it cannot be used where a value is
required at compile time. Referring to one from a `const`, from another static's
initializer, or from a pattern is a compile error.

#### Static storage is configured on the virtual machine

The values of statics are not stored in the `Unit`. A unit only records how many
statics there are, which slot each was assigned, and what each initializer is.
The values live in a separate `Globals` storage handed to the machine when it is
constructed:

```rust
let globals = Globals::new(unit.clone())?;
globals.set(["CONFIG"], rune::to_value(config)?)?;

let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
vm.call(["main"], ())?;

let counter = globals.get(["COUNTER"])?;
```

This means one compiled unit can back any number of independent states, and that
a caller can read and write statics by item without running a script. It also
supports handing scripts access to something the host owns, such as a registry
or a service handle, without threading it through the signature of every
function that needs it. See the `global_registry` example.

Static storage holds `Value`s, so it cannot be shared between threads. Each
thread builds its own `Globals` from the shared `Unit`, and cross-thread state
is handled by the value placed in the slot, for instance a host type whose
interior is an `Arc<Mutex<..>>`. See the `statics_threads` example.

`Build::build_vm` provisions storage automatically, so the shorthand path needs
no extra setup. A `Vm` constructed directly starts without storage, and reading
a static through it reports that none has been configured.

#### Statics can be declared with the build

A static doesn't have to come from the source being compiled. A collection of
`Statics` can be handed to the build, and every static declared in it is added
to the unit as if the source had declared it:

```rust
let mut statics = Statics::new();
statics.insert(["REGISTRY"])?;

let unit = rune::prepare(&mut sources)
    .with_statics(&statics)
    .build()?;
```

This lets a host hand scripts a piece of state it owns without the scripts
having to declare it, in the same way a native module makes a function available
to them. The name is an item, so `["config", "REGISTRY"]` declares the static
inside of the `config` module.

Since there is no source to evaluate, such a static has no initializer. It
starts out uninitialized just like a `static REGISTRY;` in a script does, so the
caller has to assign it before anything reads it. A source which declares an
item of the same name is a compile error. See the `declared_statics` example.

#### New API

- `rune::Statics` and `Build::with_statics`.
- `rune::runtime::Globals` and `rune::runtime::GlobalsError`.
- `Vm::with_globals`, `Vm::globals`, `Vm::globals_mut` and `Vm::is_same_globals`.
- `Unit::globals_len` and `Unit::global_slot`.
- `rune::runtime::DebugGlobal`, along with `DebugInfo::globals` and
  `DebugInfo::global`. When debug information is enabled the unit records the
  item path of each static slot, so diagnostics can name the static rather than
  report a slot number.

### Changed

- Two instructions were added, `GlobalGet` and `GlobalSet`. These are internal,
  but they show up in disassembly through `--dump-unit --emit-instructions`.
- The serialized representation of `Unit` and `DebugInfo` gained fields for
  static slots. Cached byte code produced by an earlier version will not decode.
- `Options::debug_info` is now read when recording the names of static slots. It
  was previously settable but unused.
- `Function::into_sync` produces a `SyncFunction` which cannot carry static
  storage, since the storage is not thread safe. Reading a static through a
  `SyncFunction` reports that no storage has been configured.

<!--
TODO: The entries above cover statics only. The rest of 0.15.0 is still to be
written up; there are roughly 89 commits since 0.14.0.
-->

[Statics]: https://rune-rs.github.io/book/statics.html
