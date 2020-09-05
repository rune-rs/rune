# Foreword

> "Why am I making a programming language?"

This question keeps rolling around in my head as I'm typing out the code that is
slowly shaping into *Rune*. Programming is like magic. You imagine it in your
mind, write it out, and there it is. Doing *stuff* which wasn't being done
before.

Truth be told, I'm scared that people will tell me that I'm wasting my time.
This has already been done, or "Why not just use X?". An thing so glaringly
obvious that all of my efforts are wasted.

But you actually don't need a reason. It can simply be for The [Joy of
Creating], and then it's just you spending your own time as a hobby. No harm
done. But I want to talk about why I'm making Rune beyond just for fun. So I'm
dedicating this foreword to it. Because I want others to benefit from this I'm
obligated to describe why it might matter to them.

So here's why I started making a new programming language.

## The other project

I've spent a lot of effort working on [OxidizeBot], a Twitch bot that streamers
can use to add commands and other interactive things in their chat. I built it
for myself while streaming. When adding features I always spend way to much time
tinkering with it. Making it as generic as possible so it can solve more than
just one problem. And when it's a personal project, I don't care about being
efficient. I care much more about doing things the right way.

...

Ok, I *sometimes* do that professionally as well. But a working environment is
much more constrained. Personal projects should be fun!

Anyway, that means the bot isn't incredibly specialized to only suit my needs
and can be used by others. It's starting to see a little bit of that use now
which is a lot of fun. I made something which helps people do something cool.

All the commands in the bot are written in [Rust], and [compiled straight into
the bot]. This is nice because Rust is an incredible language. But Rust is also
complex. Not nedlessly mind you. In my view it's complex because it decided to
tackle *really hard problems*. And that usually comes with a [base level of
complexity] that it's very hard to get rid of.

But it's still tricky enough that streamers who have limited programming
experience struggle getting up and running*. I wanted them to be able to write
their own commands. Ones they could just drop into a folder and *presto* -
you're up and running.

> *: To this day I've tutored two of these streamers who were interested in
> learning Rust to write their own commands.

Embedding a Rust compiler isn't feasible. So for this reason I started looking
into dynamic programming languages. Ones that could be embedded into an existing
application with little to no effort. That seemlessly integrates with its
environment.

So a number of candidates came up, and the one that stood out the most to me was
[Rhai].

So why is Rhai awesome? It has Rust-like syntax. The runtime is fully written in
mostly safe Rust, and can be easily embedded. Hooking up Rust functions is a
piece of cake.

But Rhai has a set of design decisions which didn't *exactly* scratch my itch.
And the more I used it, the more I got inspired and started thinking about
things that could be changed or added. [I contributed a bit to the project]. But
it started to dawn on me that Rhai's approach wasn't exactly what I wanted.
There's nothing wrong with this. The authors of Rhai have specific goals and
ideas of what they want to accomplish. And while I believe that it's feasible to
push Rhai in a different direction, the project probably would emerge looking
much different on the other side. So I wanted a fresh slate to find my own
compromises. To discover more freely what works and doesn't work well.

When I started working on Rune I had the following *rough* goals in mind:

* Performance should be comparable to Lua and Python (And eventually LuaJIT when
  we have cranelift).
* Scripts should compile quickly.
* Rune should feel like "Rust without types".
* Be as good as Rhai when it comes to integrating with native Rust.
* Work well through C bindings.
* A minimalistic stack-based runtime that is strictly singlethreaded*.

> *: If this feels like a step backwards to you, don't worry too much. We can
  still have concurrency and threading using async code as you'll see later in
  this book.

Rune is now in a state where I want people to poke at it. Not *too* hard mind
you, because it's still very early days. So don't use it in anything user-facing
or security critical just yet. The compiler is very much in flux and a
miscompilation will definitely cause the wrong closure to be called. You know,
the one that *doesn't* do all your fancy security checks 😅.

But the more poking and prodding people do, the more issues will be found. Every
solved issue brings Rune one step close to being production ready.

I really want to thank Jonathan Turner and all the contributors to the Rhai
project. They have been an an immense inspiration to me. If you need an embedded
scripting engine that is more mature right now than *Rune*, please do take
*Rhai* for a spin.

You can find the project at [its GitHub page]. I hope you'll enjoy it!

&mdash; John-John Tedro

[Joy of Creating]: https://en.wikipedia.org/wiki/The_Joy_of_Painting
[Rust]: https://rust-lang.org
[base level of complexity]: https://en.wikipedia.org/wiki/Waterbed_theory
[compiled straight into the bot]: https://github.com/udoprog/OxidizeBot/tree/master/bot/src/module
[OxidizeBot]: https://github.com/udoprog/OxidizeBot
[Rust]: https://rust-lang.org
[Rhai]: https://github.com/jonathandturner/rhai
[I contributed a bit to the project]: https://github.com/jonathandturner/rhai/commits?author=udoprog
[like Lua]: https://www.lua.org/pil/26.1.html
[cranelift]: https://github.com/bytecodealliance/wasmtime/tree/main/cranelift
[its GitHub page]: https://github.com/rune-rs/rune/