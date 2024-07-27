# Drop order

Rune implements the following rules when determining when a place should be
dropped.

Places are distinct from values, in that they refer to a place where a value is
stored, not the value itself.

The distinction becomes apparent when we note that the same value can be
referenced by multiple places:

```
let a = 42;
let b = a;
```

Above, `b` is a distinct place that refers to the same value as `a`.

There are two ways for a value to be dropped:
 * All the places referencing go out of scope.
 * The value is explicitly dropped with `drop(value)`.

The second variant causes the value to be dropped. Using any of the places
referencing that value after it has been dropped will cause an error.

#### Variables

A variable declaration like this:

```rune
let var = 42;
```

Defines a place called `var`.

Once variables like these go out of scope, their place is dropped. However,
dropping a place doesn't necessarily mean the value is dropped. This only
happens when that is the last place referencing that variable.

```rune
let object = {
    let var = 42;
    let object = #{ var };
};

// object can be used here and `var` is still live.
```

#### Temporaries

Temporaries are constructed when evaluating any non-trivial expression, such as
this:

```rune
let var = [42, (), "hello"];
```

The drop order for temporaries is not strictly defined and can be extended. But
never beyond the block in which they are defined.
