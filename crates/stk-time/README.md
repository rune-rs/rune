[![Build Status](https://github.com/udoprog/stk/workflows/Build/badge.svg)](https://github.com/udoprog/stk/actions)

# stk-time

The stk time package.

### Usage

Add the following to your `Cargo.toml`:

```toml
stk = "0.2"
stk-timer = "0.2"
```

Install it into your context:

```rust
let mut context = stk::Context::with_default_packages()?;
context.install(stk_time::module()?)?;
```

Use it in Rune:

```rust
use time;

fn main() {
    time::delay_for(time::Duration::from_secs(10)).await;
}
```
