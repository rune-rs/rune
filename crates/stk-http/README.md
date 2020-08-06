[![Build Status](https://github.com/udoprog/stk/workflows/Build/badge.svg)](https://github.com/udoprog/stk/actions)

# stk-http

HTTP module for stk based on reqwest.

### Usage

Add the following to your `Cargo.toml`:

```toml
stk = "0.2"
stk-http = "0.2"
# not necessary, but useful
stk-json = "0.2"
```

Install it into your context:

```rust
let mut context = stk::Context::with_default_packages()?;
context.install(stk_http::module()?)?;
context.install(stk_json::module()?)?;
```

Use it in Rune:

```rust,ignore
use http;
use json;

fn main() {
    let client = http::Client::new();
    let response = client.get("http://worldtimeapi.org/api/ip");
    let text = response.text();
    let json = json::from_string(text);

    let timezone = json["timezone"];

    if timezone is String {
        dbg(timezone);
    }

    let body = json::to_bytes(#{"hello": "world"});

    let response = client.post("https://postman-echo.com/post")
        .body_bytes(body)
        .send();

    let response = json::from_string(response.text());
    dbg(response);
}
```
