//! Comparison and ordering.

use core::cmp::Ordering;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::runtime::{Formatter, Protocol, Value, VmResult};
use crate::shared::Caller;
use crate::{ContextError, Module};

/// Comparison and ordering.
#[rune::module(::std::cmp)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    {
        let ty = m.ty::<Ordering>()?.docs(docstring! {
            /// An `Ordering` is the result of a comparison between two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            /// use std::ops::cmp;
            ///
            /// let result = 1.cmp(2);
            /// assert_eq!(Ordering::Less, result);
            ///
            /// let result = 1.cmp(1);
            /// assert_eq!(Ordering::Equal, result);
            ///
            /// let result = 2.cmp(1);
            /// assert_eq!(Ordering::Greater, result);
            /// ```
        })?;

        let mut ty = ty.make_enum(&["Less", "Equal", "Greater"])?;

        ty.variant_mut(0)?
            .make_empty()?
            .constructor(|| Ordering::Less)?
            .docs(docstring! {
                /// "An ordering where a compared value is less than another.
            })?;

        ty.variant_mut(1)?
            .make_empty()?
            .constructor(|| Ordering::Equal)?
            .docs(docstring! {
                /// "An ordering where a compared value is equal to another.
            })?;

        ty.variant_mut(2)?
            .make_empty()?
            .constructor(|| Ordering::Greater)?
            .docs(docstring! {
                /// "An ordering where a compared value is greater than another.
            })?;

        m.associated_function(
            &Protocol::IS_VARIANT,
            |this: Ordering, index: usize| match (this, index) {
                (Ordering::Less, 0) => true,
                (Ordering::Equal, 1) => true,
                (Ordering::Greater, 2) => true,
                _ => false,
            },
        )?;
    }

    m.function_meta(ordering_partial_eq__meta)?;
    m.implement_trait::<Ordering>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(ordering_eq__meta)?;
    m.implement_trait::<Ordering>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(ordering_debug_fmt)?;
    m.function_meta(min__meta)?;
    m.function_meta(max__meta)?;

    let mut t = m.define_trait(["PartialEq"])?;

    t.docs(docstring! {
        /// Trait for comparisons using the equality operator.
        ///
        /// Implementing this trait for types provides the `==` and `!=`
        /// operators for those types.
        ///
        /// `x.eq(y)` can also be written `x == y`, and `x.ne(y)` can be written
        /// `x != y`. We use the easier-to-read infix notation in the remainder
        /// of this documentation.
        ///
        /// This trait allows for comparisons using the equality operator, for
        /// types that do not have a full equivalence relation. For example, in
        /// floating point numbers `NaN != NaN`, so floating point types
        /// implement `PartialEq` but not [`trait@Eq`]. Formally speaking, when
        /// `Rhs == Self`, this trait corresponds to a [partial equivalence
        /// relation].
        ///
        /// [partial equivalence relation]:
        ///     https://en.wikipedia.org/wiki/Partial_equivalence_relation
        ///
        /// Implementations must ensure that `eq` and `ne` are consistent with
        /// each other:
        ///
        /// - `a != b` if and only if `!(a == b)`.
        ///
        /// The default implementation of `ne` provides this consistency and is
        /// almost always sufficient. It should not be overridden without very
        /// good reason.
        ///
        /// If [`PartialOrd`] or [`Ord`] are also implemented for `Self` and
        /// `Rhs`, their methods must also be consistent with `PartialEq` (see
        /// the documentation of those traits for the exact requirements). It's
        /// easy to accidentally make them disagree by deriving some of the
        /// traits and manually implementing others.
        ///
        /// The equality relation `==` must satisfy the following conditions
        /// (for all `a`, `b`, `c` of type `A`, `B`, `C`):
        ///
        /// - **Symmetry**: if `A: PartialEq<B>` and `B: PartialEq<A>`, then
        ///   **`a == b` implies `b == a`**; and
        ///
        /// - **Transitivity**: if `A: PartialEq<B>` and `B: PartialEq<C>` and
        ///   `A: PartialEq<C>`, then **`a == b` and `b == c` implies `a ==
        ///   c`**. This must also work for longer chains, such as when `A:
        ///   PartialEq<B>`, `B: PartialEq<C>`, `C: PartialEq<D>`, and `A:
        ///   PartialEq<D>` all exist.
        ///
        /// Note that the `B: PartialEq<A>` (symmetric) and `A: PartialEq<C>`
        /// (transitive) impls are not forced to exist, but these requirements
        /// apply whenever they do exist.
        ///
        /// Violating these requirements is a logic error. The behavior
        /// resulting from a logic error is not specified, but users of the
        /// trait must ensure that such logic errors do *not* result in
        /// undefined behavior. This means that `unsafe` code **must not** rely
        /// on the correctness of these methods.
        ///
        /// ## Cross-crate considerations
        ///
        /// Upholding the requirements stated above can become tricky when one
        /// crate implements `PartialEq` for a type of another crate (i.e., to
        /// allow comparing one of its own types with a type from the standard
        /// library). The recommendation is to never implement this trait for a
        /// foreign type. In other words, such a crate should do `impl
        /// PartialEq<ForeignType> for LocalType`, but it should *not* do `impl
        /// PartialEq<LocalType> for ForeignType`.
        ///
        /// This avoids the problem of transitive chains that criss-cross crate
        /// boundaries: for all local types `T`, you may assume that no other
        /// crate will add `impl`s that allow comparing `T == U`. In other
        /// words, if other crates add `impl`s that allow building longer
        /// transitive chains `U1 == ... == T == V1 == ...`, then all the types
        /// that appear to the right of `T` must be types that the crate
        /// defining `T` already knows about. This rules out transitive chains
        /// where downstream crates can add new `impl`s that "stitch together"
        /// comparisons of foreign types in ways that violate transitivity.
        ///
        /// Not having such foreign `impl`s also avoids forward compatibility
        /// issues where one crate adding more `PartialEq` implementations can
        /// cause build failures in downstream crates.
        ///
        /// # Examples
        ///
        /// ```rune
        /// let x = 0;
        /// let y = 1;
        ///
        /// assert_eq!(x == y, false);
        /// assert_eq!(x.eq(y), false);
        ///
        /// assert!((1.0).eq(1.0));
        /// assert!(!(1.0).eq(2.0));
        ///
        /// assert!(1.0 == 1.0);
        /// assert!(1.0 != 2.0);
        /// ```
    })?;

    t.handler(|cx| {
        let partial_eq = cx.find(&Protocol::PARTIAL_EQ)?;
        let partial_eq = Caller::<(Value, Value), 2, bool>::new(partial_eq);

        cx.function("ne", move |a: Value, b: Value| {
            VmResult::Ok(!vm_try!(partial_eq.call((a, b))))
        })?;

        Ok(())
    })?;

    t.function("eq")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Compare two values for equality.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.eq(2), false);
            /// assert_eq!(2.eq(2), true);
            /// assert_eq!(2.eq(1), false);
            /// ```
        })?;

    t.function("ne")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Compare two values for inequality.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.ne(2), true);
            /// assert_eq!(2.ne(2), false);
            /// assert_eq!(2.ne(1), true);
            /// ```
        })?;

    let mut t = m.define_trait(["Eq"])?;

    t.docs(docstring! {
        /// Trait for comparisons corresponding to [equivalence relations](
        /// https://en.wikipedia.org/wiki/Equivalence_relation).
        ///
        /// This means, that in addition to `a == b` and `a != b` being strict
        /// inverses, the relation must be (for all `a`, `b` and `c`):
        ///
        /// - reflexive: `a == a`;
        /// - symmetric: `a == b` implies `b == a` (required by `PartialEq` as
        ///   well); and
        /// - transitive: `a == b` and `b == c` implies `a == c` (required by
        ///   `PartialEq` as well).
        ///
        /// This property cannot be checked by the compiler, and therefore `Eq`
        /// implies [`PartialEq`], and has no extra methods.
        ///
        /// Violating this property is a logic error. The behavior resulting
        /// from a logic error is not specified, but users of the trait must
        /// ensure that such logic errors do *not* result in undefined behavior.
        /// This means that `unsafe` code **must not** rely on the correctness
        /// of these methods.
        ///
        /// Implement `Eq` in addition to `PartialEq` if it's guaranteed that
        /// `PartialEq::eq(a, a)` always returns `true` (reflexivity), in
        /// addition to the symmetric and transitive properties already required
        /// by `PartialEq`.
        /// ```
    })?;

    t.handler(|cx| {
        _ = cx.find(&Protocol::EQ)?;
        Ok(())
    })?;

    t.docs(docstring! {
        /// Trait for equality comparisons.
        ///
        /// This trait allows for comparing whether two values are equal or not.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use std::cmp::Eq;
        ///
        /// assert!(1.eq(1));
        /// assert!(!1.eq(2));
        /// ```
    })?;

    let mut t = m.define_trait(["PartialOrd"])?;

    t.docs(docstring! {
        /// Trait for types that form a [partial
        /// order](https://en.wikipedia.org/wiki/Partial_order).
        ///
        /// The `lt`, `le`, `gt`, and `ge` methods of this trait can be called
        /// using the `<`, `<=`, `>`, and `>=` operators, respectively.
        ///
        /// The methods of this trait must be consistent with each other and
        /// with those of [`PartialEq`]. The following conditions must hold:
        ///
        /// 1. `a == b` if and only if `partial_cmp(a, b) == Some(Equal)`.
        /// 2. `a < b` if and only if `partial_cmp(a, b) == Some(Less)`
        /// 3. `a > b` if and only if `partial_cmp(a, b) == Some(Greater)`
        /// 4. `a <= b` if and only if `a < b || a == b` 5. `a >= b` if and only
        /// if `a > b || a == b`
        /// 6. `a != b` if and only if `!(a == b)`.
        ///
        /// Conditions 2–5 above are ensured by the default implementation.
        /// Condition 6 is already ensured by [`PartialEq`].
        ///
        /// If [`Ord`] is also implemented for `Self` and `Rhs`, it must also be
        /// consistent with `partial_cmp` (see the documentation of that trait
        /// for the exact requirements). It's easy to accidentally make them
        /// disagree by deriving some of the traits and manually implementing
        /// others.
        ///
        /// The comparison relations must satisfy the following conditions (for
        /// all `a`, `b`, `c` of type `A`, `B`, `C`):
        ///
        /// - **Transitivity**: if `A: PartialOrd<B>` and `B: PartialOrd<C>` and
        ///   `A: PartialOrd<C>`, then `a < b` and `b < c` implies `a < c`. The
        ///   same must hold for both `==` and `>`. This must also work for
        ///   longer chains, such as when `A: PartialOrd<B>`, `B:
        ///   PartialOrd<C>`, `C: PartialOrd<D>`, and `A: PartialOrd<D>` all
        ///   exist.
        /// - **Duality**: if `A: PartialOrd<B>` and `B: PartialOrd<A>`, then `a
        ///   < b` if and only if `b > a`.
        ///
        /// Note that the `B: PartialOrd<A>` (dual) and `A: PartialOrd<C>`
        /// (transitive) impls are not forced to exist, but these requirements
        /// apply whenever they do exist.
        ///
        /// Violating these requirements is a logic error. The behavior
        /// resulting from a logic error is not specified, but users of the
        /// trait must ensure that such logic errors do *not* result in
        /// undefined behavior. This means that `unsafe` code **must not** rely
        /// on the correctness of these methods.
        ///
        /// ## Cross-crate considerations
        ///
        /// Upholding the requirements stated above can become tricky when one
        /// crate implements `PartialOrd` for a type of another crate (i.e., to
        /// allow comparing one of its own types with a type from the standard
        /// library). The recommendation is to never implement this trait for a
        /// foreign type. In other words, such a crate should do `impl
        /// PartialOrd<ForeignType> for LocalType`, but it should *not* do `impl
        /// PartialOrd<LocalType> for ForeignType`.
        ///
        /// This avoids the problem of transitive chains that criss-cross crate
        /// boundaries: for all local types `T`, you may assume that no other
        /// crate will add `impl`s that allow comparing `T < U`. In other words,
        /// if other crates add `impl`s that allow building longer transitive
        /// chains `U1 < ... < T < V1 < ...`, then all the types that appear to
        /// the right of `T` must be types that the crate defining `T` already
        /// knows about. This rules out transitive chains where downstream
        /// crates can add new `impl`s that "stitch together" comparisons of
        /// foreign types in ways that violate transitivity.
        ///
        /// Not having such foreign `impl`s also avoids forward compatibility
        /// issues where one crate adding more `PartialOrd` implementations can
        /// cause build failures in downstream crates.
        ///
        /// ## Corollaries
        ///
        /// The following corollaries follow from the above requirements:
        ///
        /// - irreflexivity of `<` and `>`: `!(a < a)`, `!(a > a)`
        /// - transitivity of `>`: if `a > b` and `b > c` then `a > c`
        /// - duality of `partial_cmp`: `partial_cmp(a, b) == partial_cmp(b,
        ///   a).map(Ordering::reverse)`
        ///
        /// ## Strict and non-strict partial orders
        ///
        /// The `<` and `>` operators behave according to a *strict* partial
        /// order. However, `<=` and `>=` do **not** behave according to a
        /// *non-strict* partial order. That is because mathematically, a
        /// non-strict partial order would require reflexivity, i.e. `a <= a`
        /// would need to be true for every `a`. This isn't always the case for
        /// types that implement `PartialOrd`, for example:
        ///
        /// ```
        /// let a = f64::sqrt(-1.0);
        /// assert_eq!(a <= a, false);
        /// ```
        ///
        /// ## How can I implement `PartialOrd`?
        ///
        /// `PartialOrd` only requires implementation of the [`PARTIAL_CMP`]
        /// protocol, with the others generated from default implementations.
        ///
        /// However it remains possible to implement the others separately for
        /// types which do not have a total order. For example, for floating
        /// point numbers, `NaN < 0 == false` and `NaN >= 0 == false` (cf. IEEE
        /// 754-2008 section 5.11).
        ///
        /// `PARTIAL_CMP` requires your type to be [`PARTIAL_EQ`].
        ///
        /// If your type is [`ORD`], you can implement [`PARTIAL_CMP`] by using
        /// [`CMP`].
        ///
        /// You may also find it useful to use [`PARTIAL_CMP`] on your type's
        /// fields.
        ///
        /// # Examples
        ///
        /// ```rune
        /// let x = 0;
        /// let y = 1;
        ///
        /// assert_eq!(x < y, true);
        /// assert_eq!(x.lt(y), true);
        /// ```
        ///
        /// [`partial_cmp`]: PartialOrd::partial_cmp
        /// [`cmp`]: Ord::cmp
    })?;

    t.handler(|cx| {
        let partial_cmp = cx.find(&Protocol::PARTIAL_CMP)?;
        let partial_cmp = Caller::<(Value, Value), 2, Option<Ordering>>::new(partial_cmp);

        cx.find_or_define(&Protocol::LT, {
            let partial_cmp = partial_cmp.clone();

            move |a: Value, b: Value| {
                let Some(o) = vm_try!(partial_cmp.call((a.clone(), b.clone()))) else {
                    return VmResult::Ok(false);
                };

                VmResult::Ok(matches!(o, Ordering::Less))
            }
        })?;

        cx.find_or_define(&Protocol::LE, {
            let partial_cmp = partial_cmp.clone();

            move |a: Value, b: Value| {
                let Some(o) = vm_try!(partial_cmp.call((a.clone(), b.clone()))) else {
                    return VmResult::Ok(false);
                };

                VmResult::Ok(matches!(o, Ordering::Less | Ordering::Equal))
            }
        })?;

        cx.find_or_define(&Protocol::GT, {
            let partial_cmp = partial_cmp.clone();

            move |a: Value, b: Value| {
                let Some(o) = vm_try!(partial_cmp.call((a.clone(), b.clone()))) else {
                    return VmResult::Ok(false);
                };

                VmResult::Ok(matches!(o, Ordering::Greater))
            }
        })?;

        cx.find_or_define(&Protocol::GE, {
            let partial_cmp = partial_cmp.clone();

            move |a: Value, b: Value| {
                let Some(o) = vm_try!(partial_cmp.call((a.clone(), b.clone()))) else {
                    return VmResult::Ok(false);
                };

                VmResult::Ok(matches!(o, Ordering::Greater | Ordering::Equal))
            }
        })?;

        Ok(())
    })?;

    t.function("partial_cmp")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Option<Ordering>>()?
        .docs(docstring! {
            /// Compare two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            ///
            /// assert_eq!(1.partial_cmp(2), Some(Ordering::Less));
            /// assert_eq!(2.partial_cmp(2), Some(Ordering::Equal));
            /// assert_eq!(2.partial_cmp(1), Some(Ordering::Greater));
            /// ```
        })?;

    t.function("lt")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Tests less than (for `self` and `other`) and is used by the `<` operator.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.0 < 1.0, false);
            /// assert_eq!(1.0 < 2.0, true);
            /// assert_eq!(2.0 < 1.0, false);
            /// ```
        })?;

    t.function("le")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Tests less than or equal to (for `self` and `other`) and is used
            /// by the `<=` operator.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.0 <= 1.0, true);
            /// assert_eq!(1.0 <= 2.0, true);
            /// assert_eq!(2.0 <= 1.0, false);
            /// ```
        })?;

    t.function("gt")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Tests greater than (for `self` and `other`) and is used by the
            /// `>` operator.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.0 > 1.0, false);
            /// assert_eq!(1.0 > 2.0, false);
            /// assert_eq!(2.0 > 1.0, true);
            /// ```
        })?;

    t.function("ge")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Tests greater than or equal to (for `self` and `other`) and is
            /// used by the `>=` operator.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.0 >= 1.0, true);
            /// assert_eq!(1.0 >= 2.0, false);
            /// assert_eq!(2.0 >= 1.0, true);
            /// ```
        })?;

    let mut t = m.define_trait(["Ord"])?;

    t.docs(docstring! {
        /// Trait for types that form a [total
        /// order](https://en.wikipedia.org/wiki/Total_order).
        ///
        /// Implementations must be consistent with the [`PartialOrd`]
        /// implementation, and ensure `max`, `min`, and `clamp` are consistent
        /// with `cmp`:
        ///
        /// - `partial_cmp(a, b) == Some(cmp(a, b))`.
        /// - `max(a, b) == max_by(a, b, cmp)` (ensured by the default
        ///   implementation).
        /// - `min(a, b) == min_by(a, b, cmp)` (ensured by the default
        ///   implementation).
        /// - For `a.clamp(min, max)`, see the [method docs](#method.clamp)
        ///   (ensured by the default implementation).
        ///
        /// It's easy to accidentally make `cmp` and `partial_cmp` disagree by
        /// deriving some of the traits and manually implementing others.
        ///
        /// Violating these requirements is a logic error. The behavior
        /// resulting from a logic error is not specified, but users of the
        /// trait must ensure that such logic errors do *not* result in
        /// undefined behavior. This means that `unsafe` code **must not** rely
        /// on the correctness of these methods.
        ///
        /// ## Corollaries
        ///
        /// From the above and the requirements of `PartialOrd`, it follows that
        /// for all `a`, `b` and `c`:
        ///
        /// - exactly one of `a < b`, `a == b` or `a > b` is true; and
        /// - `<` is transitive: `a < b` and `b < c` implies `a < c`. The same
        ///   must hold for both `==` and `>`.
        ///
        /// Mathematically speaking, the `<` operator defines a strict [weak
        /// order]. In cases where `==` conforms to mathematical equality, it
        /// also defines a strict [total order].
        ///
        /// [weak order]: https://en.wikipedia.org/wiki/Weak_ordering
        /// [total order]: https://en.wikipedia.org/wiki/Total_order
        ///
        /// ## Lexicographical comparison
        ///
        /// Lexicographical comparison is an operation with the following
        /// properties:
        ///  - Two sequences are compared element by element.
        ///  - The first mismatching element defines which sequence is
        ///    lexicographically less or greater than the other.
        ///  - If one sequence is a prefix of another, the shorter sequence is
        ///    lexicographically less than the other.
        ///  - If two sequences have equivalent elements and are of the same
        ///    length, then the sequences are lexicographically equal.
        ///  - An empty sequence is lexicographically less than any non-empty
        ///    sequence.
        ///  - Two empty sequences are lexicographically equal.
        ///
        /// ## How can I implement `Ord`?
        ///
        /// `Ord` requires that the type also be [`PARTIAL_RD`] and [`EQ`]
        /// (which requires [`PARTIAL_EQ`]).
        ///
        /// Then you must define an implementation for [`CMP`]. You may find it
        /// useful to use [`CMP`] on your type's fields.
    })?;

    t.handler(|cx| {
        let cmp = cx.find(&Protocol::CMP)?;
        let cmp = Caller::<(Value, Value), 2, Ordering>::new(cmp);

        cx.find_or_define(&Protocol::MIN, {
            let cmp = cmp.clone();

            move |a: Value, b: Value| match vm_try!(cmp.call((a.clone(), b.clone()))) {
                Ordering::Less | Ordering::Equal => VmResult::Ok(a),
                Ordering::Greater => VmResult::Ok(b),
            }
        })?;

        cx.find_or_define(&Protocol::MAX, {
            let cmp = cmp.clone();

            move |a: Value, b: Value| match vm_try!(cmp.call((a.clone(), b.clone()))) {
                Ordering::Less | Ordering::Equal => VmResult::Ok(b),
                Ordering::Greater => VmResult::Ok(a),
            }
        })?;

        Ok(())
    })?;

    t.function("cmp")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Ordering>()?
        .docs(docstring! {
            /// Compare two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            ///
            /// assert_eq!(1.cmp(2), Ordering::Less);
            /// assert_eq!(2.cmp(2), Ordering::Equal);
            /// assert_eq!(2.cmp(1), Ordering::Greater);
            /// ```
        })?;

    t.function("min")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Ordering>()?
        .docs(docstring! {
            /// Return the minimum of two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.min(2), 1);
            /// assert_eq!(2.min(2), 2);
            /// assert_eq!(2.min(1), 1);
            /// ```
        })?;

    t.function("max")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Ordering>()?
        .docs(docstring! {
            /// Return the maximum of two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.max(2), 2);
            /// assert_eq!(2.max(2), 2);
            /// assert_eq!(2.max(1), 2);
            /// ```
        })?;

    Ok(m)
}

/// Compares and returns the maximum of two values.
///
/// Returns the second argument if the comparison determines them to be equal.
///
/// Internally uses the [`CMP`] protocol.
///
/// # Examples
///
/// ```rune
/// use std::cmp::max;
///
/// assert_eq!(max(1, 2), 2);
/// assert_eq!(max(2, 2), 2);
/// ```
#[rune::function(keep)]
fn max(v1: Value, v2: Value) -> VmResult<Value> {
    VmResult::Ok(match vm_try!(Value::cmp(&v1, &v2)) {
        Ordering::Less | Ordering::Equal => v2,
        Ordering::Greater => v1,
    })
}

/// Compares and returns the minimum of two values.
///
/// Returns the first argument if the comparison determines them to be equal.
///
/// Internally uses the [`CMP`] protocol.
///
/// # Examples
///
/// ```rune
/// use std::cmp::min;
///
/// assert_eq!(min(1, 2), 1);
/// assert_eq!(min(2, 2), 2);
/// ```
#[rune::function(keep)]
fn min(v1: Value, v2: Value) -> VmResult<Value> {
    VmResult::Ok(match vm_try!(Value::cmp(&v1, &v2)) {
        Ordering::Less | Ordering::Equal => v1,
        Ordering::Greater => v2,
    })
}

/// Perform a partial ordering equality test.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
///
/// assert!(Ordering::Less == Ordering::Less);
/// assert!(Ordering::Less != Ordering::Equal);
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
fn ordering_partial_eq(this: Ordering, other: Ordering) -> bool {
    this == other
}

/// Perform a total ordering equality test.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
/// use std::cmp::Ordering;
///
/// assert!(eq(Ordering::Less, Ordering::Less));
/// assert!(!eq(Ordering::Less, Ordering::Equal));
/// ```
#[rune::function(keep, instance, protocol = EQ)]
fn ordering_eq(this: Ordering, other: Ordering) -> bool {
    this == other
}

/// Debug format [`Ordering`].
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
///
/// assert_eq!(format!("{:?}", Ordering::Less), "Less");
/// ```
#[rune::function(instance, protocol = DEBUG_FMT)]
fn ordering_debug_fmt(this: Ordering, s: &mut Formatter) -> VmResult<()> {
    vm_write!(s, "{:?}", this)
}
