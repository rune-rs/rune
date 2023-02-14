# generate

A utility project for generating code for the rest of the project.

This must be run with nightly, since it uses [Genco].

From the root of the project:

```bash
cargo +nightly run --manifest-path tools/generate/Cargo.toml
```

[Genco]: https://github.com/udoprog/genco
