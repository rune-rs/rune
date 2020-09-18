+++
template = "index.html"
page_template = "page.html"
+++

Welcome to the Rune Programming Language.

Rune is a dynamic programming language that is easily embeddable and has support for an exciting set of language features.

#### Asynchronous First

Rune prioritized excellent support for `async` with support for async functions,
[closures], [blocks], and [generators]. And native support for [`select`], a
popular control flow mechanism for asynchronous code.

```rune
struct Timeout;

async fn request(timeout) {
    let request = http::get("https://google.com");
    let timeout = time::delay_for(time::Duration::from_secs(2));

    let result = select {
        _ = timeout => Err(Timeout),
        res = request => res,
    }?;

    Ok(result)
}
```

[closures]: https://rune-rs.github.io/book/async.html#async-closures
[blocks]: https://rune-rs.github.io/book/async.html#async-blocks
[generators]: https://rune-rs.github.io/book/streams.html
[`select`]: https://rune-rs.github.io/book/async.html#select-blocks
