# Sandboxing

Rune is capable of enforcing the following types of limitations:
* Memory limiting, where you specify the maxium amount of memory that Rune may
  use either during compilation or execution.
* Instruction budgeting, where you can specify how many instructions the virtual
  machine is permitted to execute.

## Instruction budgeting

Instruction budgeting is performed using the [`with` function] in the
`rune::budget` module.

The `with` function is capable of wrapping functions and futures. When wrapping
a future it ensures that the budget is suspended appropriately with the
execution of the future.

Budgeting is only performed on a per-instruction basis in the virtual machine.
What exactly constitutes an instruction might be a bit vague. But important to
note is that without explicit co-operation from native functions the budget
cannot be enforced. So care must be taken with the native functions that you
provide to Rune to ensure that the limits you impose cannot be circumvented.

[`with` function]: https://docs.rs/rune/latest/rune/runtime/budget/fn.with.html

## Memory limiting

Memory limiting is performed using the [`with` function] in the
`rune::alloc::limit` module.

```rust
use rune::alloc::limit;
use rune::alloc::Vec;

let f = limit::with(1024, || {
    let mut vec = Vec::<u32>::try_with_capacity(256)?;

    for n in 0..256u32 {
        vec.try_push(n)?;
    }

    Ok::<_, rune_alloc::Error>(vec.into_iter().sum::<u32>())
});

let sum = f.call()?;
assert_eq!(sum, 32640);
```

In order for memory limiting to work as intended, you're may only use the
collections provided in the [`rune::alloc`] module. These contain forks of
popular collections such as [`std::collections`] and [`hashbrown`].

The `with` function is capable of wrapping [functions] and [futures]. When
wrapping a future it ensures that the limit is suspended appropriately with the
execution of the future.

[`with` function]: https://docs.rs/rune/latest/rune/alloc/limit/fn.with.html
[`rune::alloc`]: https://docs.rs/rune/latest/rune/alloc/index.html
[`std::collections`]: https://doc.rust-lang.org/std/collections/index.html
[`hashbrown`]: docs.rs/hashbrown
[functions]: 
[futures]: 
