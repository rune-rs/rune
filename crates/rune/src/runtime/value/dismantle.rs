//! Taking a graph of values apart without recursing into it.

use core::fmt;
use core::mem::replace;

use crate::runtime::{Inline, RawAnyGuard, Value};

use super::Repr;

/// How many values can be waiting before the worklist has to grow.
///
/// Taking a value apart is done without allocating for as long as few enough of
/// its values are waiting at once, which is what nearly every value dropped by
/// a destructor looks like.
const INLINE: usize = 8;

/// How deeply the walk is nested, so that the tests can check that a graph is
/// taken apart without re-entering it.
#[cfg(test)]
mod counters {
    use core::cell::Cell;

    std::thread_local! {
        static DEPTH: Cell<usize> = const { Cell::new(0) };
        pub(super) static NESTING: Cell<usize> = const { Cell::new(0) };
    }

    /// Count a level of nesting for as long as it is held.
    pub(super) struct Level;

    impl Level {
        pub(super) fn new() -> Self {
            let depth = DEPTH.with(|d| {
                d.set(d.get() + 1);
                d.get()
            });

            NESTING.with(|n| n.set(n.get().max(depth)));
            Level
        }
    }

    impl Drop for Level {
        fn drop(&mut self) {
            DEPTH.with(|d| d.set(d.get() - 1));
        }
    }

    /// Reset what has been counted, returning what it was.
    pub(super) fn take() -> usize {
        NESTING.with(|n| n.replace(0))
    }
}

/// Where the values taken out of a graph wait to be taken apart themselves.
///
/// A value which is made of other values is a graph which is built at runtime,
/// so how deep it is has nothing to do with how deep the source was. Dropping
/// the outermost value of such a graph drops the values it is made of, which
/// drops what *they* are made of, and so on, once per level - a chain of a few
/// thousand values is enough to exhaust the call stack.
///
/// The walk therefore takes the values out of one value at a time and leaves
/// them waiting here rather than descending into them, so it costs memory
/// rather than native frames.
///
/// # Memory
///
/// This is the one place in the runtime which allocates infallibly, through
/// [`rust_alloc`] rather than through the memory limit in effect. Taking a
/// value apart is what *frees* memory, so failing because memory has run out
/// leaves nothing which could be done about it, and a destructor has nobody to
/// report the failure to.
///
/// What it takes is bounded by the graph being taken apart, which the limit
/// already bounds: what waits here came out of the graph and is a fraction of
/// what it occupied. Few enough values fit without allocating at all, which is
/// what a destructor handed an ordinary value costs.
pub(crate) struct Worklist {
    /// The values which are waiting, for as long as there are few enough of
    /// them to keep without allocating.
    inline: [Repr; INLINE],
    /// How many of `inline` are in use.
    len: usize,
    /// Where the values which do not fit are kept.
    spilled: rust_alloc::vec::Vec<Repr>,
}

impl fmt::Debug for Worklist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Worklist")
            .field("len", &(self.len + self.spilled.len()))
            .field("spilled", &self.spilled.capacity())
            .finish()
    }
}

impl Worklist {
    /// Construct an empty worklist, which does not allocate until it is handed
    /// more values at once than it can keep.
    pub(crate) const fn new() -> Self {
        Self {
            inline: [const { Repr::Inline(Inline::Empty) }; INLINE],
            len: 0,
            spilled: rust_alloc::vec::Vec::new(),
        }
    }

    /// How much the worklist has grown beyond what it holds without
    /// allocating, which is what a machine keeps rather than growing one for
    /// every value it takes apart.
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.spilled.capacity()
    }

    /// Take `value` apart, and every value it is made of.
    ///
    /// This takes the values out of one value at a time and leaves them
    /// waiting, so the walk costs the memory that takes rather than the native
    /// frames which descending into them would cost.
    #[inline]
    pub(crate) fn dismantle(&mut self, value: Value) {
        let repr = value.take_repr();

        // Values which cannot contain other values are the common case and
        // need nothing to take apart.
        if matches!(repr, Repr::Inline(..)) {
            return;
        }

        self.dismantle_repr(repr);
    }

    /// Take the value out of `slot`, leaving it empty, and take it apart.
    #[inline]
    pub(crate) fn take(&mut self, slot: &mut Value) {
        self.dismantle(Value::take(slot));
    }

    /// Write `value` into `slot`, taking apart the value which was there rather
    /// than leaving it to its destructor.
    #[inline]
    pub(crate) fn replace(&mut self, slot: &mut Value, value: Value) {
        self.dismantle(replace(slot, value));
    }

    /// Take every value in `values` apart over the same worklist.
    ///
    /// This is what an operation which destroys many values at once - clearing
    /// a collection, tearing down a call frame - uses, so that they share the
    /// memory the walk grows rather than each growing its own.
    pub(crate) fn dismantle_all<'a>(&mut self, values: impl IntoIterator<Item = &'a mut Value>) {
        for slot in values {
            self.take(slot);
        }
    }

    /// Take apart a value which has already been taken out of the [`Value`]
    /// which held it, which is what a destructor has in hand.
    pub(crate) fn dismantle_repr(&mut self, repr: Repr) {
        #[cfg(test)]
        let _level = self::counters::Level::new();

        let mut current = repr;

        loop {
            // Everything inside is taken out and left waiting, so dropping what
            // remains cannot descend into anything.
            handover(&mut current, &mut Handover { work: self });
            drop(current);

            let Some(next) = self.pop() else {
                break;
            };

            current = next;
        }
    }

    /// Leave a value waiting until it is taken apart.
    fn push(&mut self, repr: Repr) {
        if self.len < INLINE {
            self.inline[self.len] = repr;
            self.len += 1;
            return;
        }

        // Growing is what a graph which is deep or wide costs, and it is not
        // charged to the memory limit - see the type documentation.
        self.spilled.push(repr);
    }

    /// The value which has been waiting the shortest, since taking the most
    /// recent one apart first is what keeps the walk from holding more of the
    /// graph at once than it has to.
    fn pop(&mut self) -> Option<Repr> {
        if let Some(repr) = self.spilled.pop() {
            return Some(repr);
        }

        let len = self.len.checked_sub(1)?;
        self.len = len;
        Some(replace(&mut self.inline[len], Repr::Inline(Inline::Empty)))
    }
}

/// Take the values which `repr` is made of out of it and hand them over.
fn handover(repr: &mut Repr, out: &mut Handover<'_>) {
    match repr {
        Repr::Inline(..) => {}
        Repr::Dynamic(sequence) => {
            // Another reference is keeping the values alive, so dropping this
            // one does not descend into them.
            if !sequence.is_last_owner() {
                return;
            }

            if let Ok(mut values) = sequence.borrow_mut() {
                out.consume_all(values.iter_mut());
            }
        }
        // Which values an externally typed value is made of is only known to
        // the type, so it is asked - see [`Dismantle`].
        Repr::Any(any) => any.dismantle(out),
    }
}

/// The values being taken out of a graph, handed to the value which is made of
/// them so that it can hand them over.
///
/// Handing a value over takes it out of where it was and leaves it waiting to be
/// taken apart in its own right, which is what keeps the walk from descending -
/// and therefore from costing a native frame for every level a graph nests.
pub struct Handover<'a> {
    work: &'a mut Worklist,
}

impl Handover<'_> {
    /// Hand over everything `from` is made of.
    ///
    /// This is what a type calls for each of the values it holds, whether that
    /// is a [`Value`] or something which is itself made of values.
    #[inline]
    pub fn consume<T>(&mut self, from: &mut T)
    where
        T: ?Sized + Dismantle,
    {
        from.dismantle(self);
    }

    /// Hand over everything in `values`.
    #[inline]
    pub fn consume_all<'a, T>(&mut self, values: impl IntoIterator<Item = &'a mut T>)
    where
        T: 'a + ?Sized + Dismantle,
    {
        for value in values {
            self.consume(value);
        }
    }

    /// Hand over the value which `guard` keeps alive, if it still keeps one
    /// alive.
    ///
    /// An iterator over a collection holds the collection through a guard
    /// rather than in a slot it could hand over, so this is how it hands the
    /// collection over.
    #[inline]
    pub fn consume_ref(&mut self, guard: &mut RawAnyGuard) {
        let Some(owner) = guard.take_value() else {
            return;
        };

        self.push(owner);
    }

    /// Hand `value` over as it is.
    #[inline]
    pub fn push(&mut self, value: Value) {
        let repr = value.take_repr();

        // A value which cannot contain other values is dropped here and now,
        // since dropping it cannot descend into anything.
        if matches!(repr, Repr::Inline(..)) {
            return;
        }

        self.work.push(repr);
    }
}

/// How a value stored in the virtual machine hands over the values it is made
/// of, so that a graph of them is taken apart without recursing into it.
///
/// Every type which implements [`Any`] implements this as well, and the [`Any`]
/// derive writes it: a type which is not made of values hands nothing over and
/// is dropped in place, which is what the derive writes unless it is told
/// otherwise.
///
/// A type which *does* hold [`Value`]s has to hand them over, since a script can
/// nest such a type inside itself without any bound - `v = [v]` in a loop - and
/// dropping what that builds costs a native frame per level otherwise, which
/// exhausts the call stack. Mark the fields which hold them:
///
/// ```
/// use rune::Any;
/// use rune::Value;
///
/// #[derive(Any)]
/// struct Pair {
///     #[rune(dismantle)]
///     first: Value,
///     #[rune(dismantle)]
///     second: Value,
///     count: u32,
/// }
/// ```
///
/// Anything the derive cannot write - a collection, an iterator which holds what
/// it walks through a guard - is written by hand instead, by declaring the type
/// `#[rune(dismantle)]` and implementing this trait for it.
///
/// [`Any`]: derive@crate::Any
pub trait Dismantle {
    /// Hand over every value this is made of, leaving nothing behind which is
    /// itself made of values.
    ///
    /// Whatever is left is dropped once this returns, so a value which is not
    /// handed over is dropped where it is - which costs a native frame for
    /// every level a script nests it.
    fn dismantle(&mut self, out: &mut Handover<'_>);
}

/// A value is the leaf of the walk: it is handed over as it is, unless it
/// cannot contain other values, in which case there is nothing to hand over.
///
/// This is what makes handing a field over the same call whatever the field
/// holds, which is what the derive writes.
impl Dismantle for Value {
    #[inline]
    fn dismantle(&mut self, out: &mut Handover<'_>) {
        if self.is_inline() {
            return;
        }

        out.push(Value::take(self));
    }
}

/// A value which may or may not be there is made of the one it holds, and a
/// script can nest one inside another - `a = Some(a)` in a loop.
impl Dismantle for Option<Value> {
    #[inline]
    fn dismantle(&mut self, out: &mut Handover<'_>) {
        out.consume_all(self.iter_mut());
    }
}

/// Either outcome is made of the value it holds, and a script can nest one
/// inside another - `a = Ok(a)` in a loop.
impl Dismantle for Result<Value, Value> {
    #[inline]
    fn dismantle(&mut self, out: &mut Handover<'_>) {
        let (Ok(slot) | Err(slot)) = self;
        out.consume(slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::alloc::String;
    use crate::runtime::{Object, OwnedTuple, Vec};
    use crate::support::Result;
    use crate::Any;

    /// How deep the graphs built below are.
    ///
    /// Taking them apart by recursing overflowed in the thousands, and a test
    /// runs on a thread with a much smaller stack than the main one, so this is
    /// well past what used to abort the process.
    ///
    /// Under miri a graph that size is out of reach, so what runs there is only
    /// deep enough to reach every shape a level can be. It still says the walk
    /// does not nest - the counter below is what says that, at any depth - but
    /// it no longer says the walk survives a graph the native stack could not.
    const DEEP: usize = if cfg!(miri) { 128 } else { 50000 };

    /// How many values the graph built below holds at one level.
    ///
    /// Taking a value apart used to walk over the values it had already handed
    /// over to find the next one, which is quadratic - this is enough of them
    /// that the test does not finish if that comes back.
    ///
    /// Under miri it is only enough to walk a container which holds more than a
    /// handful, which says the walk is right rather than that it is one pass.
    const WIDE: usize = if cfg!(miri) { 512 } else { 200000 };

    /// A type which says which of its fields hold values the way an external
    /// type does, so that what the derive writes is walked over as well.
    #[derive(Any)]
    #[rune(crate)]
    struct Holder {
        #[rune(dismantle)]
        value: Value,
        #[allow(unused)]
        count: i64,
    }

    /// Build a graph which is `depth` deep, of every shape which has to be
    /// taken apart, and hand back the value it is built around.
    ///
    /// Every level branches, so that more than one value has to be handed over
    /// at each of them.
    fn deep(depth: usize) -> Result<(Value, Value)> {
        let innermost = Value::try_from(Vec::new())?;
        let mut value = innermost.clone();

        for n in 0..depth {
            let mut vec = Vec::new();
            vec.push(Value::try_from(Vec::new())?)?;
            vec.push(Value::from(n as i64))?;
            vec.push(value)?;

            value = match n % 4 {
                0 => Value::try_from(vec)?,
                1 => {
                    let mut object = Object::new();
                    object.insert(String::try_from("a")?, Value::try_from(vec)?)?;
                    object.insert(String::try_from("b")?, Value::from(n as i64))?;
                    Value::try_from(object)?
                }
                2 => Value::new(Holder {
                    value: Value::try_from(vec)?,
                    count: n as i64,
                })?,
                _ => Value::try_from(OwnedTuple::try_from(rust_alloc::vec![
                    Value::try_from(vec)?,
                    Value::from(n as i64),
                ])?)?,
            };
        }

        Ok((value, innermost))
    }

    /// Taking a deep graph apart is what a destructor does, and it never
    /// re-enters itself no matter how deep the graph is.
    #[test]
    fn dropping_takes_a_deep_graph_apart() -> Result<()> {
        let (value, innermost) = deep(DEEP)?;

        let _ = self::counters::take();

        drop(value);

        let nesting = self::counters::take();

        assert_eq!(nesting, 1, "Taking a graph apart should not nest");

        let any = innermost.as_any().expect("Innermost value should be any");
        assert!(any.is_last_owner(), "Graph should be gone");
        Ok(())
    }

    /// The machine takes the same graph apart over the worklist it keeps.
    #[test]
    fn dismantling_takes_a_deep_graph_apart() -> Result<()> {
        let (value, innermost) = deep(DEEP)?;

        Worklist::new().dismantle(value);

        let any = innermost.as_any().expect("Innermost value should be any");
        assert!(any.is_last_owner(), "Graph should be gone");
        Ok(())
    }

    /// A value which is made of very many others is taken apart in one pass over
    /// them rather than in one pass for each of them.
    ///
    /// A map cannot remove the entry it hands over, so a walk which looked for
    /// the next value to hand over from the start every time was quadratic in
    /// how many the map held.
    #[test]
    fn taking_a_wide_graph_apart_is_one_pass() -> Result<()> {
        let mut object = Object::new();

        for n in 0..WIDE {
            let key = String::try_from(rust_alloc::format!("k{n}").as_str())?;
            // A value which is made of other values, so that it is handed over
            // rather than dropped where it is.
            object.insert(key, Value::try_from(Vec::new())?)?;
        }

        drop(Value::try_from(object)?);
        Ok(())
    }

    /// Taking apart a value which holds few enough values does not allocate,
    /// which is what a destructor handed an ordinary value costs.
    #[test]
    fn taking_a_small_graph_apart_does_not_allocate() -> Result<()> {
        let mut vec = Vec::new();
        vec.push(Value::try_from(Vec::new())?)?;
        vec.push(Value::try_from(Vec::new())?)?;

        let mut work = Worklist::new();
        work.dismantle(Value::try_from(vec)?);

        assert_eq!(
            work.capacity(),
            0,
            "Taking a small value apart should not grow the worklist"
        );

        Ok(())
    }
}
