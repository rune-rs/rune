# Workspaces and Packages

Workspaces in Rune are an optional feature to help you organize larger
collections of Rune packages. To create a workspaces, you need to create
a `Rune.toml` manifest that tells Rune where to find the projects which
comprise the workspace:

```toml
[workspace]
members = [
    "package-a",
    "nested/package-b"
]
```

The names of the members will be interpreted as directories relative to the
workspace `Rune.toml` manifest. Each workspace member also needs to have its
own `Rune.toml` manifest to give the package a name:

```toml
[package]
name = "a"
version = "0.0.0"
```

Within a package, Rune will automatically detect different types of compilation
targets based on the directory structure of the package. This is similar to the
## Declarations bundled by cargo crates

A Rust crate which embeds Rune, or provides modules for it, can ship
declaration sources describing what it installs and name them in its
`Cargo.toml`:

```toml
[package.metadata.rune]
declarations = ["rune/*.d.rn"]
```

Each entry is a path relative to the crate's manifest directory, in which `*`
matches any part of a file or directory name. The language server finds the
cargo project it was opened in by the nearest `Cargo.toml` at or above the
directory the editor opened, runs `cargo metadata --offline` for it, and
compiles what every package in it names beside each script it builds - those of
a workspace and loose scripts alike. Only crates that cargo knows about are
looked at, so a dependency fetched from a registry has to include the files in
what it publishes.

Cargo is asked once when the server starts and again whenever a `Cargo.toml` is
saved or reported changed by the editor. If its dependencies cannot be resolved
offline it is asked about the workspace alone. A missing or failing `cargo`, one
which takes longer than ten seconds, or a malformed `[package.metadata.rune]`
is logged as a warning and does not stop the server.

[Rust package layout]:

- Source code goes in the `src` directory.
- The default executable is `src/main.rn`, and will be named the same as the
  package.
- The default library file is `src/lib.rn`. Libraries differ from executables
  in that they don't have a `main` function and cannot be invoked using
  `rune run`.
- Other executables can be placed in `src/bin/`.
- Benchmarks go in the `benches/` directory.
- Examples go in the `examples/` directory.
- Tests go in the `tests/` directory.

If a binary, example, bench, or test consists of multiple source files, place
a `main.rn` file along with the extra modules within a subdirectory of the
respective directory. The name of the executable will be the directory name.

## Example

Here's a sample workspace layout that might go along with the manifest files
above:

```text
workspace
├── Rune.toml <-- Workspace manifest
├── package-a
│   ├── Rune.toml <-- Package 'a' manifest
│   ├── src
│   │   ├── bin
│   │   │   ├── multi-file-executable
│   │   │   │   ├── main.rn
│   │   │   │   └── module
│   │   │   │       └── mod.rn
│   │   │   └── named-executable.rn
│   │   ├── lib.rn
│   │   └── main.rn
│   ├── benches
│   │   ├── collatz.rn
│   │   └── multi-file-bench
│   │       ├── collatz.rn
│   │       └── main.rn
│   ├── examples
│   │   ├── multi-file-example
│   │   │   ├── lib.rn
│   │   │   └── main.rn
│   │   └── simple.rn
│   └── tests
│       ├── fire
│       │   ├── lib.rn
│       │   └── main.rn
│       └── smoke.rn
└── nested
    └── package-b
        ├── Rune.toml <-- Package 'b' manifest
        ├── src
        │   ├── bin
        │   │   ├── multi-file-executable
        │   │   │   ├── main.rn
        │   │   │   └── module
        │   │   │       └── mod.rn
        │   │   └── named-executable.rn
        │   ├── lib.rn
        │   └── main.rn
        ├── benches
        │   ├── collatz.rn
        │   └── multi-file-bench
        │       ├── collatz.rn
        │       └── main.rn
        ├── examples
        │   ├── multi-file-example
        │   │   ├── lib.rn
        │   │   └── main.rn
        │   └── simple.rn
        └── tests
            ├── fire
            │   ├── lib.rn
            │   └── main.rn
            └── smoke.rn
```

You don't need to pay too much attention to the `package-b` tree, since it's
identical to `package-a`. The only reason it's there is to demonstrate how Rune
handles duplicate names within a workspace.

If we call `rune run` in the `workspace` directory, we'll see the following:

```text
     Running bin `package-a/src/main.rn` (from a)
     Running bin `package-a/src/bin/named-executable.rn` (from a)
     Running bin `package-a/src/bin/multi-file-executable/main.rn` (from a)
     Running bin `nested/package-b/src/main.rn` (from b)
     Running bin `nested/package-b/src/bin/named-executable.rn` (from b)
     Running bin `nested/package-b/src/bin/multi-file-executable/main.rn` (from b)
     Running example `package-a/examples/simple.rn` (from a)
     Running example `package-a/examples/multi-file-example/main.rn` (from a)
     Running example `nested/package-b/examples/simple.rn` (from b)
     Running example `nested/package-b/examples/multi-file-example/main.rn` (from b)
... output from the various executables
```

Note that `rune run` only executes the binaries (both default and named) and
examples from both workspace packages. We can filter this down even further
by calling, for example, `rune run --bin named-executable`, which shows this:

```text
     Running bin `package-a/src/bin/named-executable.rn` (from a)
     Running bin `nested/package-b/src/bin/named-executable.rn` (from b)
... output from the various executables
```

(Similar filters exist for `--lib`, `--example`, `--bench`, and `--test`.)

If we're only interested in targets from one of the workspace packages, it's
possible to restrict the list of targets using the `--package` or `-p` command
line flag:

```text
$ rune run --bin named-executable -p a
     Running bin `package-a/src/bin/named-executable.rn` (from a)
... output
```

We can get a fuller view of all of the entry points that Rune sees by calling
something like `rune check`, which shows the following:

```text
    Checking bin `package-a/src/main.rn` (from a)
    Checking bin `package-a/src/bin/named-executable.rn` (from a)
    Checking bin `package-a/src/bin/multi-file-executable/main.rn` (from a)
    Checking bin `nested/package-b/src/main.rn` (from b)
    Checking bin `nested/package-b/src/bin/named-executable.rn` (from b)
    Checking bin `nested/package-b/src/bin/multi-file-executable/main.rn` (from b)
    Checking lib `package-a/src/lib.rn` (from a)
    Checking lib `nested/package-b/src/lib.rn` (from b)
    Checking test `package-a/tests/smoke.rn` (from a)
    Checking test `package-a/tests/fire/main.rn` (from a)
    Checking test `nested/package-b/tests/smoke.rn` (from b)
    Checking test `nested/package-b/tests/fire/main.rn` (from b)
    Checking example `package-a/examples/simple.rn` (from a)
    Checking example `package-a/examples/multi-file-example/main.rn` (from a)
    Checking example `nested/package-b/examples/simple.rn` (from b)
    Checking example `nested/package-b/examples/multi-file-example/main.rn` (from b)
    Checking bench `package-a/benches/collatz.rn` (from a)
    Checking bench `package-a/benches/multi-file-bench/main.rn` (from a)
    Checking bench `nested/package-b/benches/collatz.rn` (from b)
    Checking bench `nested/package-b/benches/multi-file-bench/main.rn` (from b)
Checking: package-a/src/main.rn
Checking: ...
...
```

Note that while Rune will list entry points by their top-level file, for
operations such as `check` and `fmt`, the entire tree of modules will be
considered. If we introduce an error in a module file and try to run
`rune check --bin multi-file-executable`, we'll get the following:

```text
    Checking bin `package-a/src/bin/multi-file-executable/main.rn` (from a)
    Checking bin `nested/package-b/src/bin/multi-file-executable/main.rn` (from b)
Checking: package-a/src/bin/multi-file-executable/main.rn
error: Missing macro module::printline
  ┌─ package-a/src/bin/multi-file-executable/module/mod.rn:2:5
  │
2 │     printline!("running a/multi-file-executable")
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Missing macro module::printline
```

## Declarations

A package's own [declaration sources](./declarations.md), every `*.d.rn` file
under its directory, are compiled beside each of its entry points, which is how
a project describes the host its scripts are run in. A `.d.rn` file is never an
entry point itself, even under `src/bin`.

## Dependencies

A package can name other packages it depends on in a `[dependencies]` section
of its manifest. The only kind of dependency there is a path dependency: a
directory, relative to the manifest, which holds a `Rune.toml` of its own with a
`[package]` in it.

```toml
[package]
name = "app"
version = "0.0.0"

[dependencies]
host = { path = "../host" }
```

What a dependency brings is its [declaration sources](./declarations.md): every
`*.d.rn` file under its directory, along with those of whatever it depends on in
turn. Directories whose names start with `.`, `target`, and directories holding
a package of their own are not looked in. These are compiled beside each entry
point of the depending package by `rune check`, `rune run`, `rune test` and
`rune bench`, and by the language server, so a script can call what a host
library declares without the host being there.

A dependency is not a member of the workspace: none of its entry points are
checked or run, and a `[workspace]` in its manifest is left alone. Anything
other than a path dependency - a version, a `git` source, a key beside `path` -
is reported in the manifest where it is written, as is a path without a
`Rune.toml` and a package which ends up depending on itself.

[Rust package layout]: https://doc.rust-lang.org/cargo/guide/project-layout.html