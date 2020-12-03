# Benchmarks for Rune

Site: https://rune-rs.github.io/rune/dev/bench/

You can run a benchmark by:

```sh
cargo bench
```

## Generating flamegraphs

Install [`cargo-profile`] (since [`flamegraph` can't run benchmarks] easily):

```sh
cargo install cargo-profile
```

Run a single benchmark to generate a `flamegraph.svg` file:

```sh
cargo profile flamegraph bench --bench <bench>
```

[`cargo-profile`]: https://github.com/kdy1/cargo-profile
[`flamegraph` can't run benchmarks]: https://github.com/flamegraph-rs/flamegraph/issues/80
