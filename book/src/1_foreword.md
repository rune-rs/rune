# Foreword

> "Why am I building a programming language? Is it even possible?"

This question keeps rolling around in my head as I'm typing out the code that is
slowly shaping into *Rune*. Programming is like magic. You imagine it in your
mind, write it out, and there it is. Doing *stuff* which wasn't being done
before.

Truth be told, I'm scared that people will tell me that I'm wasting my time.
This has already been done, or "Why not just use X?". An X so glaringly obvious
that all of my efforts are wasted.

But you actually don't need a reason. It can simply be for The Joy of Creating,
and then it's just you spending your own time as a hobby. No harm done.

Anyway, here's why im making Rune.

## The other project

I've spent a lot of effort working on is [OxidizeBot], a Twitch bot that
streamers can use to add commands and other interactive things in their chat.
I built it for myself while streaming, but I'm also an aggressive generaliser.
When adding features I always spend way to much time tinkering with it.
Making it as generic as possible, so it can solve more than just one problem.

And when it's a personal project, I just don't care about being efficient.
I care much more deeply about doing things the right way.

...

Ok, I *sometimes* do that professionally as well.

Anyway, that means the bot isn't incredibly specialized to my needs and could
be used by others. It's starting to see a little bit of that use now, which is a
lot of fun. I made something which helps people to something cool.

All the commands in the bot are written in [Rust], and compiled straight into
the bot. This is nice because Rust is an incredible language. But Rust is also
complex. Not nedlessly mind you. In my view it's complex because it decided to
tackle *really hard problems*, so it's about as complex as it has to be.

But it's tricky enough that streamers who have very little programming
experience struggle getting up and running*. Because of this I wanted to provide
a way for these users to write their own dynamic command handlers. Ones they
could just drop into a folder and *presto* - you're up and running.

> I've personally tutored at least two of these streamers who were interested in
> learning Rust, but then you're getting further away from writing chat
> commands.

For this reason I started looking into dynamic programming languages. Ones that
could be embedded into an existing Rust application with little to no effort.
That seemlessly integrates with the Rust environment.

If you're one of those thinking that type systems are the best thing since
sliced bread. Then I'm just gonna say **yes**, you're right. But type systems
are also one of these complexities which can get in the way of being productive
if you're not familiar with them. Not everyone wants to learn how to climb
before they can walk.

So number of candidates came up, and the one that stood out the most to me was
[Rhai].

So why is Rhai awesome? It has Rust-like syntax. The runtime is fully written in
mostly safe Rust, and can be easily embedded. Hooking up Rust functions is a
piece of cake.

But of course Rhai has a set of features and design constraints which didn't
exactly scratch my itch. And the more I used it, the more I thought about things
that could be changed or added.
I [contributes a bit to the project][rhai-contribs].
But it also started to dawn on my that Rhai's particular set of constraints
weren't entirely in line with what I wanted to use.

In no particular order. I want a stack-based virtual machine that you could
write low-complexity C-ffi bindings for by interacting solely with a stack
[like Lua][lua-bindings]. Compiling the language down closer to machine
instructions would hopefully make it feasible to generate native code through
[cranelift] through the same. And finally opinionated first-class support for
asynchronous programming.

While I believe that these are feasible in Rhai, I also think the project would
emerge looking much differently on the other side. So I want a fresh slate of
design constraints to find my own compromises. To discover more freely what
works and doesn't work well.

So I set out to make Rune. When I started I had the following goals in mind:

* Performance should be comparable to Lua (Not LuaJIT).
* Scripts should compile quickly.
* What would Rust look like if it was a dynamic programming language without
  type annotations? Rune should feel like taking a Rust program and removing
  type annotations.
* Be as good or better than Rhai when it comes to integrating with native Rust.
* Make sure Rune's internals will work well through future C bindings.
* A lightweight runtime that is strictly singlethreaded. Concurrency can be
  accomplished by using async functions (who might themselves use threads).

That's it.

Rune is now in a state where I want people to poke at it. Not *too* hard mind
you, because it's still very early days. So please don't use it in anything
user-facing or security critical. A miscompilation will cause the wrong closure
to be called. You know, the one which *doesn't* perform your security checks 😅.

But the more poking and prodding people do, the more issues will be found. Every
solved issue brings Rune one step close to being production ready.

I really want to thank Jonathan Turner and the Rhai project. They have been a
hugely inspirational to me. If you need an embedded scripting engine that is
more mature right now than *Rune*, please take *Rhai* for a spin!

&mdash; John-John Tedro

[OxidizeBot]: https://github.com/udoprog/OxidizeBot
[Rust]: https://rust-lang.org
[Rhai]: https://github.com/jonathandturner/rhai
[rhai-contribs]: https://github.com/jonathandturner/rhai/commits?author=udoprog
[lua-bindings]: https://www.lua.org/pil/26.1.html
[cranelift]: https://github.com/bytecodealliance/wasmtime/tree/main/cranelift