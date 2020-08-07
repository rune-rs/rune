[![Build Status](https://github.com/udoprog/runestick/workflows/Build/badge.svg)](https://github.com/udoprog/runestick/actions)

# runestick-time

The runestick time package.

### Usage

Add the following to your `Cargo.toml`:

```toml
runestick = "0.2"
runestick-time = "0.2"
```

Install it into your context:

```rust
let mut context = runestick::Context::with_default_packages()?;
context.install(runestick_time::module()?)?;
```

Use it in Rune:

```rust
use time;

fn main() {
    time::delay_for(time::Duration::from_secs(10)).await;
}
```
