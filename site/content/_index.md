+++
template = "index.html"
page_template = "page.html"
+++

Welcome to the Rune Programming Language.

Rune is a dynamic programming language that is easily embeddable and has support for an exciting set of language features.

#### Asynchronous first

Rune prioritized excellent support for `async` with support for async functions,
[closures], [blocks], and [generators]. And native support for [`select`], a
popular control flow mechanism for asynchronous code.

{% rune(footnote = "Asynchronous programming using select", manually = true) %}
use std::future;

struct Timeout;

async fn request(timeout) {
    let request = http::get(`https://httpstat.us/200?sleep={timeout}`);
    let timeout = time::delay_for(time::Duration::from_secs(1));

    let result = select {
        res = request => res,
        _ = timeout => Err(Timeout),
    }?;

    let text = result.text().await?;
    Ok(text)
}

async fn main() {
    let result = future::join((request(0), request(1500))).await;
    dbg(result);
}
{% end %}

[closures]: https://rune-rs.github.io/book/async.html#async-closures
[blocks]: https://rune-rs.github.io/book/async.html#async-blocks
[generators]: https://rune-rs.github.io/book/streams.html
[`select`]: https://rune-rs.github.io/book/async.html#select-blocks
