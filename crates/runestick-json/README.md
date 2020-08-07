[![Build Status](https://github.com/udoprog/runestick/workflows/Build/badge.svg)](https://github.com/udoprog/runestick/actions)

# runestick-json

The json package, providing access to functions to serialize and deserialize
json.

### Usage

Add the following to your `Cargo.toml`:

```toml
runestick = "0.2"
runestick-json = "0.2"
```

Install it into your context:

```rust
let mut context = runestick::Context::with_default_packages()?;
context.install(runestick_json::module()?)?;
```

Use it in Rune:

```rust,ignore
use json;

fn main() {
    let data = json::from_string("{\"key\": 42}");
    dbg(data);
}
```
