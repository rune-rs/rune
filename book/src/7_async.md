# Asynchronous Programming

Rune has first class support for Rust-like asynchronous programming.
In this section we'll be briefly covering what asynchronous programming is, and
how it applies to Rune as a dynamic programming language.

## What is it?

Asynchronous code allows us to run multiple tasks concurrently, and work with
the result of those tasks.

A typical example would be if we want to perform multiple HTTP requests at once:

```rust,noplaypen
fn main() {
    let a = http::get("https://google.com");
    let b = http::get("https://amazon.com");

    loop {
        let res = select {
            res = a => res?,
            res = b => res?,
        };

        match res {
            () => break,
            result => {
                println(`{result.status()}`);
            }
        }
    }
}
```

```bash
$> cargo run -- scripts/book/7_async_http.rn
200 OK
200 OK
== Unit (591.0319ms)
```

In the above code we send two requests *concurrently*. They are both processed
at the same time and we collect the result.

## `select` blocks

A fundamental construct of async programming in Rune is the `select` block.
It enables us to wait on a set of futures at the same time.

A simple example of this is if we were to implement a simple request with a
timeout:

```rust,noplaypen
struct Timeout;

fn request(timeout) {
    let request = http::get(`http://httpstat.us/200?sleep={timeout}`);
    let timeout = time::delay_for(time::Duration::from_secs(2));

    let result = select {
        _ = timeout => Err(Timeout),
        res = request => res,
    }?;

    println(`{result.status()}`);
    Ok(())
}

fn main() {
    if let Err(Timeout) = request(1000) {
        println("Request timed out!");
    }

    if let Err(Timeout) = request(4000) {
        println("Request timed out!");
    }
}
```

```bash
$> cargo run -- scripts/book/7_async_http_timeout.rn
200 OK
Request timed out!
== Unit (3.2231404s)
```

But wait, this is taking three seconds. We're not running the requests
concurrently any longer!

Well, while the request and the *timeout* is run concurrently, the `request`
function is run one at-a-time.

To fix this we need two new things: `async` functions and `.await`.

## `async` functions

`async` functions are just like regular functions, except that when called they
produce a `Future`.

In order to get the result of this `Future` it must be `.await`-ed.

```rust,noplaypen
struct Timeout;

async fn request(timeout) {
    let request = http::get(`http://httpstat.us/200?sleep={timeout}`);
    let timeout = time::delay_for(time::Duration::from_secs(2));

    let result = select {
        _ = timeout => Err(Timeout),
        res = request => res,
    }?;

    Ok(result)
}

fn main() {
    for result in [request(1000), request(4000)].await {
        match result {
            Ok(result) => println(`Result: {result.status()}`),
            Err(Timeout) => println("Request timed out!"),
        }
    }
}
```

```bash
$> cargo run -- scripts/book/7_async_http_concurrent.rn
Result: 200 OK
Request timed out!
== Unit (2.0028603s)
```

If you've been using future in Rust, one thing immediately pops out to you.

We're using `.await` in a non-`async` function!

Well, in Rune the virtual machine already comes with a Runtime. Every function
therefore has the ability to `.await` a future, regardless of if the function
itself is async or not.

In fact, the whole Runtime is asynchronous, but that is for a future, much much
more advanced chapter!