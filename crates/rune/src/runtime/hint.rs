/// The most a size hint is allowed to reserve up front.
///
/// A hint is a claim about how much is coming, and a claim is not a
/// measurement: the iterator protocol lets a script return whatever it likes,
/// and a `.rnc` read off disk carries whatever was written into it. Reserving
/// what one asks for turns a wrong number into an allocation nothing needed,
/// which is the whole cost of the operation before any of it has happened.
///
/// Growing from here costs an amortized doubling, which is what a collection
/// does anyway.
const MAX_HINT_CAPACITY: usize = 1024;

/// Clamp a size hint down to what is worth reserving up front.
#[inline]
pub(crate) fn hint_capacity(hint: usize) -> usize {
    hint.min(MAX_HINT_CAPACITY)
}
