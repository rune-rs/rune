# Foreword

> "Why am I building a programming language? Is it even possible?"

This question keeps rolling around in my head as I'm typing out the code that is
slowly becoming a new *thing*.
Programming itself is like magic, you imagine it in your mind, write it out, and
there it is.
Doing *stuff* which wasn't being done before.

Truth be told, I'm scared that people will tell me that I'm wasting my time.
This has already been done, or "Why not just use X?".
Something so glaringly obvious that all of my efforts are in vain.

You actually don't need a reason.
It can simply be for The Joy of Creating Something, and then it's just you
spending your own time as a hobby.
No harm done.

Anyway, here's why im making Rune.

## The other project

Another project I've spent a lot of effort working on is [OxidizeBot], a Twitch
bot that a streamer can use to add commands and other interactive things in
their chat.
Originally I just built it for myself, while streaming, but I'm also an
aggressive generaliser.
When I add features I always think about how it can be as generic as possible to
suit potentially many needs.

And when it's a personal project, the propensity in me is to the detriment of
keeping it simple.

...

Ok, confession. I *sometimes* do that professionally as well.
But I'm much less critical when doing personal projects.

Anyway, that meant the bot could actually be used by others.
It's starting to see a little bit of use now.
Which is actually *a lot* of fun.

So all the commands in the bot are written in [Rust], and compiled straight into
the bot.
This is nice, because Rust is an incredible language.
But Rust is also complex.
Not nedlessly, it's complex because it decided to tackle *really hard problems*.
To my best understanding, it's about as complex as it has to be.
*Most of the time*.

But it's complex enough that streamers who have very little programming
experience struggle getting up and running*.
Because of this I wanted to provide a way for them to write their own dynamic
command handlers.
Once they could just drop into a folder and *presto* - you're up and running.

> I've actually personally tutored at least two of these streamers who were
> interested in learning Rust, but then you're getting further away from writing
> chat commands.

For this reason I started looking into dynamic programming languages.
Once that could be embedded into an existing Rust application with little to no
effort.


If you're one of those thinking that type systems are the best thing since
sliced bread. Then I'm just gonna say **yes**, you are right.
But type systems are also one of these complexities which can get in the way of
being productive.
And not everyone wants to learn how to climb before they can walk.

A number of candidates came up, but this isn't a review, so I'm just gonna jump
to the one I chose and not dwell too much on the reasons: [Rhai].

So why is Rhai awesome:

* It has Rust-like syntax.
* The runtime is fully written in mostly safe Rust, and can be embedded.

Initially I wasn't going to write my own programming language.
I added support for rhai, and things worked really well.

But there were a few things that I wanted out of Rhai, and I was gearing up to
[contribute those to the project][rhai-contribs].

So the things I wanted, beyond what was already provided by Rhai was:

* A stack-based virtual machine that you could write low-complexity C-ffi
  bindings for, [like Lua][lua-bindings].
* Opinionated support for asynchronous programming.
* A future cranelift backend.

And while I believe that these are feasible in Rhai, I also think the project
would emerge as a different one on the other side.
And I wanted a fresh slate of design constraints to find my own compromises.
To discover freely what works and doesn't work well.

That being said, Rhai has been a huge inspiration for Rune.
If you need an embedded scripting engine that is more mature right now than
Rune, please go check it out!

&mdash; John-John Tedro

[OxidizeBot]: https://github.com/udoprog/OxidizeBot
[Rust]: https://rust-lang.org
[Rhai]: https://github.com/jonathandturner/rhai
[rhai-contribs]: https://github.com/jonathandturner/rhai/commits?author=udoprog
[lua-bindings]: https://www.lua.org/pil/26.1.html