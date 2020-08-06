[![Build Status](https://github.com/udoprog/stk/workflows/Build/badge.svg)](https://github.com/udoprog/stk/actions)

# stk-json

The json package, providing access to functions to serialize and deserialize
json.

### Usage

Add the following to your `Cargo.toml`:

```toml
stk = "0.2"
stk-json = "0.2"
```

Install it into your context:

```rust
let mut context = stk::Context::with_default_packages()?;
context.install(stk_json::module()?)?;
```

Use it in Rune:

```rust,ignore
use json;

fn main() {
    let data = json::from_string("{\"key\": 42}");
    dbg(data);
}
```
