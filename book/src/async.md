# Asynchronous programming

Rune has first class support for Rust-like asynchronous programming.
In this section we'll be briefly covering what asynchronous programming is, and
how it applies to Rune as a dynamic programming language.

## What is it?

Asynchronous code allows us to run multiple tasks concurrently, and work with
the result of those tasks.

A typical example would be if we want to perform multiple HTTP requests at once:

```rune
{{#include ../../scripts/book/async/async_http.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/async/async_http.rn
200 OK
200 OK
== () (591.0319ms)
```

In the above code we send two requests *concurrently*. They are both processed
at the same time and we collect the result.

## `select` blocks

A fundamental construct of async programming in Rune is the `select` block.
It enables us to wait on a set of futures at the same time.

A simple example of this is if we were to implement a simple request with a
timeout:

```rune
{{#include ../../scripts/book/async/async_http_timeout.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/async/async_http_timeout.rn
200 OK
Request timed out!
== () (3.2231404s)
```

But wait, this is taking three seconds. We're not running the requests
concurrently any longer!

Well, while the request and the *timeout* is run concurrently, the `request`
function is run one at-a-time.

To fix this we need two new things: `async` functions and `.await`.

## `async` functions

`async` functions are just like regular functions, except that when called they
produce a `Future`.

In order to get the result of this `Future` it must be `.await`ed. And `.await`
is only permitted inside of `async` functions and closures.

```rune
{{#include ../../scripts/book/async/async_http_concurrent.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/async/async_http_concurrent.rn
Result: 200 OK
Request timed out!
== () (2.0028603s)
```

## `async` closures

Closures can be prefixed with the `async` keyword, meaning calling them will
produce a future.

```rune
{{#include ../../scripts/book/async/async_closure.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/async/async_closure.rn
Status: 200 OK
== () (165.4817ms)
```

## `async` blocks

Blocks can be marked with `async` to produce on-the-fly futures. These blocks
can capture variables the same way as closures do, but take no arguments.

```rune
{{#include ../../scripts/book/async/async_blocks.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/async/async_blocks.rn
Status: 200 OK
== () (179.9381ms)
```
