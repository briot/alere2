use std::cmp::{Ordering, PartialOrd};
use crate::bounds::Bound;
use crate::multi_intervals::MultiInterval;
use crate::nothing_between::NothingBetween;

//extern crate proc_macro;
//use proc_macro::{TokenStream, TokenTree};
//
//#[proc_macro]
//pub fn intv(input: TokenStream) -> TokenStream {
//    let mut iter = input.intoiter();
//    let lower = iter.next().unwrap();
//    let comma = iter.next().unwrap();
//    assert!(matches!(comma, TokenTree::Punct(',')));
//    let upper = iter.next().unwrap();
//    match iter.next() {
//        None => Interval::new_closed_open(lower, comma),
//        TokenTree::Punct(',') => {
//            let typ = iter.next().unwrap();
//        }
//        _ => panic!("Invalid arguments for macro"),
//    }

//    let mut source = input.to_string();
//
//    // If it starts with `- ` then get rid of the extra space
//    // to_string will put a space between tokens
//    if source.starts_with("- ") {
//        source.remove(1);
//    }
//
//    let decimal = if source.contains('e') || source.contains('E') {
//        match Decimal::from_scientific(&source[..]) {
//            Ok(d) => d,
//            Err(e) => panic!("{}", e),
//        }
//    } else {
//        match Decimal::from_str_exact(&source[..]) {
//            Ok(d) => d,
//            Err(e) => panic!("{}", e),
//        }
//    };
//
//    let unpacked = decimal.unpack();
//    expand(
//        unpacked.lo,
//        unpacked.mid,
//        unpacked.hi,
//        unpacked.negative,
//        unpacked.scale,
//    )
//        let expanded = quote! {
//        ::rust_decimal::Decimal::from_parts(#lo, #mid, #hi, #negative, #scale)
//    };
//    expanded.into()
//}

/// An interval of values.
// ??? Should T be an associated type instead ?
pub struct Interval<T> {
    lower: Bound<T>,
    upper: Bound<T>,
}

impl<T> Interval<T> {
    /// Construct a left-closed, right-open intervals (`[A,B)`)
    pub fn new_closed_open(lower: T, upper: T) -> Self {
        Self {
            lower: Bound::LeftOf(lower),
            upper: Bound::LeftOf(upper),
        }
    }

    /// Construct a left-closed, right-closed intervals (`[A,B]`)
    pub fn new_closed_closed(lower: T, upper: T) -> Self {
        Self {
            lower: Bound::LeftOf(lower),
            upper: Bound::RightOf(upper),
        }
    }

    /// Construct a left-open, right-open intervals (`(A,B)`)
    pub fn new_open_open(lower: T, upper: T) -> Self {
        Self {
            lower: Bound::RightOf(lower),
            upper: Bound::LeftOf(upper),
        }
    }

    /// Construct a left-open, right-closed intervals (`(A,B]`)
    pub fn new_open_closed(lower: T, upper: T) -> Self {
        Self {
            lower: Bound::RightOf(lower),
            upper: Bound::RightOf(upper),
        }
    }

    /// Construct a left-unbounded, right-closed intervals (`(,B]`)
    pub fn new_unbounded_closed(upper: T) -> Self {
        Self {
            lower: Bound::LeftUnbounded,
            upper: Bound::RightOf(upper),
        }
    }

    /// Construct a left-unbounded, right-open intervals (`(,B)`)
    pub fn new_unbounded_open(upper: T) -> Self {
        Self {
            lower: Bound::LeftUnbounded,
            upper: Bound::LeftOf(upper),
        }
    }

    /// Construct a left-closed, right-unbounded intervals (`[A,)`)
    pub fn new_closed_unbounded(lower: T) -> Self {
        Self {
            lower: Bound::LeftOf(lower),
            upper: Bound::RightUnbounded,
        }
    }

    /// Construct a left-open, right-unbounded intervals (`(A,)`)
    pub fn new_open_unbounded(lower: T) -> Self {
        Self {
            lower: Bound::RightOf(lower),
            upper: Bound::RightUnbounded,
        }
    }

    /// Construct a doubly unbounded intervals (`(,)`) that contains all
    /// possible values.
    pub fn doubly_unbounded() -> Self {
        Self {
            lower: Bound::LeftUnbounded,
            upper: Bound::RightUnbounded,
        }
    }

    /// Returns an empty interval.  Note that there are multiple representations
    /// for empty interval, though they are all equivalent.
    pub fn empty() -> Self {
        Self {
            lower: Bound::RightUnbounded,
            upper: Bound::LeftUnbounded,
        }
    }

    /// The lower bound.  Returns None for an unbounded interval (i.e. lower
    /// is -infinity).
    /// For an empty interval, it returns whatever what used to create the
    /// interval (None if you used [`Interval::empty()`]), but the value is
    /// irrelevant.
    pub fn lower(&self) -> Option<&T> {
        self.lower.value()
    }

    /// Whether the lower bound is part of the interval.
    /// Return false for an empty interval, or if lower bound is -infinity.
    pub fn lower_inclusive(&self) -> bool {
        matches!(self.lower, Bound::LeftOf(_))
    }

    /// True if the lower bound is infinite  
    pub fn lower_unbounded(&self) -> bool {
        matches!(self.lower, Bound::LeftUnbounded)
    }

    /// The upper bound.  Returns None for an unbounded interval (i.e. upper
    /// is +infinity).
    /// For an empty interval, it returns whatever what used to create the
    /// interval (None if you used [`Interval::empty()`]), but the value is
    /// irrelevant.
    pub fn upper(&self) -> Option<&T> {
        self.upper.value()
    }

    /// Whether the upper bound is part of the interval.
    /// Return false for an empty interval, or if upper bound is +infinity.
    pub fn upper_inclusive(&self) -> bool {
        matches!(self.upper, Bound::RightOf(_))
    }

    /// True if the upper bound is infinite  
    pub fn upper_unbounded(&self) -> bool {
        matches!(self.upper, Bound::RightUnbounded)
    }

    /// Converts from `Interval<T>` to `Interval<&T>`
    pub fn as_ref(&self) -> Interval<&T> {
        Interval {
            lower: self.lower.as_ref(),
            upper: self.upper.as_ref(),
        }
    }
}

impl<T: PartialOrd + NothingBetween> Interval<T> {
    /// Whether value is contained in the interval
    pub fn contains(&self, value: &T) -> bool {
        self.lower.left_of(value) && self.upper.right_of(value)
    }

    /// Whether self contains all values of the second interval (and possibly
    /// more).
    pub fn contains_interval(&self, other: &Self) -> bool {
        other.is_empty()
            || (self.lower <= other.lower && other.upper <= self.upper)
    }

    /// True if the interval contains no element.
    /// This highly depends on how the NothingBetween trait was implemented.
    ///
    /// For instance, for f32, we consider the numbers as representable on
    /// the machine.  So an interval like:
    /// `[1.0, 1.0 + f32::EPSILON)`
    /// is empty, since we cannot represent any number from this interval.
    ///
    /// ```
    ///    use rust_intervals::Interval;
    ///    assert!(Interval::new_open_open(1.0, 1.0 + f32::EPSILON)
    ///        .is_empty());
    /// ```
    ///
    /// But if you implement your own wrapper type as
    /// ```
    ///     use rust_intervals::NothingBetween;
    ///     #[derive(PartialEq, PartialOrd)]
    ///     struct Real(f32);
    ///     impl NothingBetween for Real {
    ///         fn nothing_between(&self, _other: &Self) -> bool {
    ///             false
    ///         }
    ///     }
    /// ```
    /// then the same interval `[Real(1.0), Real(1.0 + f32::EPSILON)]` is
    /// no longer empty, even though we cannot represent any number from this
    /// interval.
    pub fn is_empty(&self) -> bool {
        match self.upper.partial_cmp(&self.lower) {
            None => true, //  can't compare bounds
            Some(Ordering::Equal | Ordering::Less) => true,
            Some(Ordering::Greater) => false,
        }
    }

    /// Whether the two intervals contain the same set of values
    pub fn equivalent(&self, other: &Self) -> bool {
        if self.is_empty() {
            other.is_empty()
        } else if other.is_empty() {
            false
        } else {
            self.lower == other.lower && self.upper == other.upper
        }
    }

    /// Whether every value in self is strictly less than (<) X
    /// (returns True is if self is empty).
    /// ```txt
    ///    [------] .
    ///             X    => strictly left of the interval
    /// ```
    pub fn strictly_left_of(&self, x: &T) -> bool {
        self.is_empty() || self.upper.left_of(x)
    }

    /// Whether every value in self is less than (<=) X.
    /// (returns True is if self is empty).
    /// ```txt
    ///    [------]
    ///           X    => left of the interval (but not strictly left of)
    /// ```
    pub fn left_of(&self, x: &T) -> bool {
        self.is_empty() || self.upper <= Bound::RightOf(x)
    }

    /// Whether every value in self is strictly less than (<) every value in
    /// right (returns True if either interval is empty).
    pub fn strictly_left_of_interval(&self, right: &Self) -> bool {
        self.is_empty() || right.is_empty() || self.upper <= right.lower
    }

    /// Whether X is strictly less than (<) every value in self.
    /// (returns True is if self is empty).
    /// ```txt
    ///    . [------]
    ///    X           => strictly right of the interval
    /// ```
    pub fn strictly_right_of(&self, x: &T) -> bool {
        self.is_empty() || self.lower.right_of(x)
    }

    /// Whether X is less than (<=) every value in self.
    /// (returns True is if self is empty).
    /// ```txt
    ///      [------]
    ///      X           => right of the interval (but not strictly right of)
    /// ```
    pub fn right_of(&self, x: &T) -> bool {
        self.is_empty() || self.lower >= Bound::LeftOf(x)
    }
}

impl<T: PartialEq + NothingBetween> Interval<T> {
    /// True if self is of the form `[A, A]`.
    /// This returns false for any other kind of interval, even if they
    /// happen to contain a single value.
    /// ```
    /// use rust_intervals::Interval;
    /// assert!(!Interval::new_open_open(0, 2).is_single());
    /// ```
    pub fn is_single(&self) -> bool {
        match (&self.lower, &self.upper) {
            (Bound::LeftOf(lp), Bound::RightOf(rp)) => *lp == *rp,
            _ => false,
        }
    }
}

impl<T: Default> Default for Interval<T> {
    /// Returns an empty interval
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Clone> Interval<T> {
    /// Returns an interval that contains a single value (`[value,value]`)
    pub fn new_single(value: T) -> Self {
        Interval::new_closed_closed(value.clone(), value)
    }
}

impl<T: PartialOrd + NothingBetween + Clone> Interval<T> {
    /// Returns the convex hull of the two intervals, i.e. the smallest
    /// interval that contains the values of both intervals.
    pub fn convex_hull(&self, right: &Self) -> Self {
        if self.is_empty() {
            right.clone()
        } else if right.is_empty() {
            self.clone()
        } else {
            Self {
                lower: self.lower.min(&right.lower),
                upper: self.upper.max(&right.upper),
            }
        }
    }

    /// Returns the result of removing all values in right from self.
    pub fn difference(&self, right: &Self) -> MultiInterval<T> {
        if self.is_empty() || right.is_empty() {
            MultiInterval::One(self.clone())
        } else {
            MultiInterval::new_from_two(
                Interval {
                    lower: self.lower.clone(),
                    upper: right.lower.min(&self.upper),
                },
                Interval {
                    lower: right.upper.max(&self.lower),
                    upper: self.upper.clone(),
                },
            )
        }
    }

    /// Returns the values that are in either of the intervals, but not
    /// both.
    pub fn symmetric_difference(&self, right: &Self) -> MultiInterval<T> {
        if self.is_empty() || right.is_empty() {
            MultiInterval::new_from_two(self.clone(), right.clone())
        } else {
            MultiInterval::new_from_two(
                Interval {
                    lower: self.lower.min(&right.lower),
                    upper: self
                        .lower
                        .max(&right.lower)
                        .min(&self.upper.min(&right.upper)),
                },
                Interval {
                    lower: self
                        .upper
                        .min(&right.upper)
                        .max(&self.lower.max(&right.lower)),
                    upper: self.upper.max(&right.upper),
                },
            )
        }
    }

    /// Whether the two intervals overlap, i.e. have at least one point in
    /// common
    pub fn intersects(&self, right: &Self) -> bool {
        !self.is_empty()
            && !right.is_empty()
            && self.lower < right.upper
            && right.lower < self.upper
    }

    /// Returns the intersection of the two intervals.  This is the same as the
    /// [`&`] operator.
    pub fn intersection(&self, right: &Self) -> Self {
        Interval {
            lower: self.lower.max(&right.lower),
            upper: self.upper.min(&right.upper),
        }
    }

    /// Returns the largest interval contained in the convex hull, that
    /// doesn't intersect with either self or right.
    /// This is empty if either of the two intervals is empty.
    /// If none of the intervals is empty, this consists of all values that
    /// are strictly between the given intervals
    pub fn between(&self, right: &Self) -> Self {
        if self.is_empty() || right.is_empty() {
            Interval::empty()
        } else {
            Interval {
                lower: self.upper.min(&right.upper),
                upper: self.lower.max(&right.lower),
            }
        }
    }

    /// If neither interval is empty, returns true if no value lies between
    /// them.  True if either of the intervals is empty.
    pub fn contiguous(&self, right: &Self) -> bool {
        if self.is_empty() || right.is_empty() {
            true
        } else {
            self.lower <= right.upper && right.lower <= self.upper
        }
    }

    /// Returns the union of the two intervals, if they are contiguous.
    /// If not, returns None.
    pub fn union(&self, right: &Self) -> Option<Self> {
        if self.contiguous(right) {
            Some(self.convex_hull(right))
        } else {
            None
        }
    }
}

///  &Interval ^ &Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitXor<&Interval<T>>
    for &Interval<T>
{
    type Output = MultiInterval<T>;

    fn bitxor(self, rhs: &Interval<T>) -> Self::Output {
        self.symmetric_difference(rhs)
    }
}

///  &Interval ^ Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitXor<Interval<T>>
    for &Interval<T>
{
    type Output = MultiInterval<T>;

    fn bitxor(self, rhs: Interval<T>) -> Self::Output {
        self.symmetric_difference(&rhs)
    }
}

///  Interval ^ Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitXor<Interval<T>>
    for Interval<T>
{
    type Output = MultiInterval<T>;

    fn bitxor(self, rhs: Interval<T>) -> Self::Output {
        self.symmetric_difference(&rhs)
    }
}

///  Interval ^ &Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitXor<&Interval<T>>
    for Interval<T>
{
    type Output = MultiInterval<T>;

    fn bitxor(self, rhs: &Interval<T>) -> Self::Output {
        self.symmetric_difference(rhs)
    }
}

///  &Interval & &Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitAnd<&Interval<T>>
    for &Interval<T>
{
    type Output = Interval<T>;

    fn bitand(self, rhs: &Interval<T>) -> Self::Output {
        self.intersection(rhs)
    }
}

///  &Interval & Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitAnd<Interval<T>>
    for &Interval<T>
{
    type Output = Interval<T>;

    fn bitand(self, rhs: Interval<T>) -> Self::Output {
        self.intersection(&rhs)
    }
}

///  Interval & Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitAnd<Interval<T>>
    for Interval<T>
{
    type Output = Interval<T>;

    fn bitand(self, rhs: Interval<T>) -> Self::Output {
        self.intersection(&rhs)
    }
}

///  Interval & &Interval
impl<T: PartialOrd + NothingBetween + Clone> std::ops::BitAnd<&Interval<T>>
    for Interval<T>
{
    type Output = Interval<T>;

    fn bitand(self, rhs: &Interval<T>) -> Self::Output {
        self.intersection(rhs)
    }
}

///   &Interval - &Interval
impl<T: PartialOrd + NothingBetween + Clone> core::ops::Sub<&Interval<T>>
    for &Interval<T>
{
    type Output = MultiInterval<T>;

    /// Same as [`Interval::difference()`]
    fn sub(self, rhs: &Interval<T>) -> Self::Output {
        self.difference(rhs)
    }
}

///   Interval - &Interval
impl<T: PartialOrd + NothingBetween + Clone> core::ops::Sub<&Interval<T>>
    for Interval<T>
{
    type Output = MultiInterval<T>;

    /// Same as [`Interval::difference()`]
    fn sub(self, rhs: &Interval<T>) -> Self::Output {
        self.difference(rhs)
    }
}

///   &Interval - Interval
impl<T: PartialOrd + NothingBetween + Clone> core::ops::Sub<Interval<T>>
    for &Interval<T>
{
    type Output = MultiInterval<T>;

    /// Same as [`Interval::difference()`]
    fn sub(self, rhs: Interval<T>) -> Self::Output {
        self.difference(&rhs)
    }
}

///   Interval - Interval
impl<T: PartialOrd + NothingBetween + Clone> core::ops::Sub<Interval<T>>
    for Interval<T>
{
    type Output = MultiInterval<T>;

    /// Same as [`Interval::difference()`]
    fn sub(self, rhs: Interval<T>) -> Self::Output {
        self.difference(&rhs)
    }
}

impl<T: Clone> std::clone::Clone for Interval<T> {
    fn clone(&self) -> Self {
        Self {
            lower: self.lower.clone(),
            upper: self.upper.clone(),
        }
    }
}

impl<T: PartialOrd + NothingBetween> PartialEq for Interval<T> {
    /// True if the two intervals contain the same values (though they might
    /// have different bounds).
    fn eq(&self, other: &Self) -> bool {
        self.equivalent(other)
    }
}

impl<T: ::core::fmt::Debug + NothingBetween + PartialOrd> ::core::fmt::Debug
    for Interval<T>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        if self.is_empty() {
            write!(f, "empty")?;
        } else {
            write!(f, "({:?},{:?})", self.lower, self.upper)?;
        }
        Ok(())
    }
}

impl<T: ::core::fmt::Display + NothingBetween + PartialOrd> ::core::fmt::Display
    for Interval<T>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        if self.is_empty() {
            write!(f, "empty")?;
        } else {
            match &self.lower {
                Bound::LeftUnbounded => write!(f, "(")?,
                Bound::LeftOf(p) => write!(f, "[{}", p)?,
                Bound::RightOf(p) => write!(f, "({}", p)?,
                Bound::RightUnbounded => panic!("Invalid left bound"),
            }
            match &self.upper {
                Bound::LeftUnbounded => panic!("Invalid right bound"),
                Bound::LeftOf(p) => write!(f, ", {})", p)?,
                Bound::RightOf(p) => write!(f, ", {}]", p)?,
                Bound::RightUnbounded => write!(f, ",)")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ::core::fmt::Debug;

    // In the world of real, there is always something in-between, even if
    // we cannot represent it.  However, in this case we may have an interval
    // for which is_empty() return false, but which actually contain no
    // values, e.g.  (A, A + f32::EPSILON)
    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    struct Mathf32(f32);
    impl NothingBetween for Mathf32 {
        fn nothing_between(&self, _other: &Self) -> bool {
            false
        }
    }

    fn assert_equivalent<T: PartialOrd + NothingBetween + Debug>(
        left: &Interval<T>,
        right: &Interval<T>,
    ) {
        assert_eq!(left, right);
        assert_eq!(right, left);
        assert!(left.equivalent(right), "{left:?} equivalent to {right:?}");
        assert!(right.equivalent(left), "{right:?} equivalent to {left:?}");
    }
    fn assert_not_equivalent<T: PartialOrd + NothingBetween + Debug>(
        left: &Interval<T>,
        right: &Interval<T>,
    ) {
        assert_ne!(left, right);
        assert_ne!(right, left);
        assert!(!left.equivalent(right));
        assert!(!right.equivalent(left));
    }

    #[test]
    fn test_contains() {
        let empty = Interval::empty();

        let intv = Interval::new_closed_open(1, 10); // [1,10)
        assert!(intv.contains(&1));
        assert!(intv.contains(&2));
        assert!(intv.contains(&9));
        assert!(!intv.contains(&10));
        assert!(!intv.contains(&11));
        assert!(intv.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv));

        let intv2 = Interval::new_closed_closed(1, 5); // [1,5]
        assert!(intv2.contains(&1));
        assert!(intv2.contains(&5));
        assert!(!intv2.contains(&6));
        assert!(intv2.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv2));
        assert!(intv.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv));

        let intv3 = Interval::new_unbounded_closed(10); // (,10]
        assert!(intv3.contains(&0));
        assert!(intv3.contains(&9));
        assert!(intv3.contains(&10));
        assert!(!intv3.contains(&11));
        assert!(intv3.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv3));
        assert!(intv3.contains_interval(&intv));
        assert!(!intv.contains_interval(&intv3));
        assert!(intv3.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv3));

        let intv4 = Interval::new_unbounded_open(10); // (,10)
        assert!(intv4.contains(&0));
        assert!(intv4.contains(&9));
        assert!(!intv4.contains(&10));
        assert!(!intv4.contains(&11));
        assert!(intv4.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv4));
        assert!(intv4.contains_interval(&intv));
        assert!(!intv.contains_interval(&intv4));
        assert!(intv4.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv4));
        assert!(intv3.contains_interval(&intv4));
        assert!(!intv4.contains_interval(&intv3));

        let intv5 = Interval::new_closed_unbounded(1); // [1,)
        assert!(!intv5.contains(&0));
        assert!(intv5.contains(&1));
        assert!(intv5.contains(&10));
        assert!(intv5.contains(&11));
        assert!(intv5.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv5));
        assert!(intv5.contains_interval(&intv));
        assert!(!intv.contains_interval(&intv5));
        assert!(intv5.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv5));
        assert!(!intv3.contains_interval(&intv5));
        assert!(!intv5.contains_interval(&intv3));
        assert!(!intv4.contains_interval(&intv5));
        assert!(!intv5.contains_interval(&intv4));

        let intv6 = Interval::doubly_unbounded();
        assert!(intv6.contains(&0));
        assert!(intv6.contains(&1));
        assert!(intv6.contains(&10));
        assert!(intv6.contains(&11));
        assert!(intv6.contains_interval(&empty));
        assert!(!empty.contains_interval(&intv6));
        assert!(intv6.contains_interval(&intv));
        assert!(!intv.contains_interval(&intv6));
        assert!(intv6.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv6));
        assert!(!intv3.contains_interval(&intv6));
        assert!(intv6.contains_interval(&intv3));
        assert!(!intv4.contains_interval(&intv6));
        assert!(intv6.contains_interval(&intv4));
        assert!(!intv5.contains_interval(&intv6));
        assert!(intv6.contains_interval(&intv5));

        // An interval with not comparable bounds is always empty
        let intv7 = Interval::new_closed_open(1.0, f32::NAN);
        assert!(!intv7.contains(&1.0));
    }

    #[test]
    fn test_inclusive() {
        let intv = Interval::new_closed_open(1, 10);
        assert_eq!(intv.lower(), Some(&1));
        assert!(intv.lower_inclusive());
        assert_eq!(intv.upper(), Some(&10));
        assert!(!intv.upper_inclusive());

        let intv = Interval::new_closed_closed(1, 10);
        assert_eq!(intv.lower(), Some(&1));
        assert!(intv.lower_inclusive());
        assert_eq!(intv.upper(), Some(&10));
        assert!(intv.upper_inclusive());

        let intv = Interval::<f32>::doubly_unbounded();
        assert_eq!(intv.lower(), None);
        assert!(!intv.lower_inclusive());
        assert_eq!(intv.upper(), None);
        assert!(!intv.upper_inclusive());

        let intv = Interval::<f32>::new_open_unbounded(1.0); //  (1,)
        assert_eq!(intv.lower(), Some(&1.0));
        assert!(!intv.lower_inclusive());
        assert_eq!(intv.upper(), None);
        assert!(!intv.upper_inclusive());

        let intv = Interval::<f32>::new_unbounded_closed(10.0); //  (,10.0]
        assert_eq!(intv.lower(), None);
        assert!(!intv.lower_inclusive());
        assert_eq!(intv.upper(), Some(&10.0));
        assert!(intv.upper_inclusive());

        let intv = Interval::<f32>::empty();
        assert_eq!(intv.lower(), None); //  matches postgres
        assert!(!intv.lower_inclusive());
        assert_eq!(intv.upper(), None); //  matches postgres
        assert!(!intv.upper_inclusive());

        let empty2 = Interval::new_open_closed(3, 3);
        assert_eq!(empty2.lower(), Some(&3)); //  doesn't match postgres
        assert!(!empty2.lower_inclusive());
        assert_eq!(empty2.upper(), Some(&3)); //  doesn't match postgres
        assert!(empty2.upper_inclusive());

        let intv = Interval::<f32>::new_single(1.0);
        assert_eq!(intv.lower(), Some(&1.0));
        assert!(intv.lower_inclusive());
        assert_eq!(intv.upper(), Some(&1.0));
        assert!(intv.upper_inclusive());
    }

    #[test]
    fn test_empty() {
        assert!(!Interval::new_closed_open(1, 10).is_empty());
        assert!(Interval::new_closed_open(1, 1).is_empty());
        assert!(Interval::new_closed_open(1, 0).is_empty());

        let empty = Interval::<f32>::empty();
        assert!(empty.is_empty());
        assert!(!empty.contains(&1.1));

        let empty2 = Interval::new_closed_open(10.0_f32, 10.0);
        assert_eq!(empty, empty2);

        assert!(Interval::new_closed_open(1.0, 1.0).is_empty());
        assert!(!Interval::new_closed_closed(1.0, 1.0).is_empty());
        assert!(Interval::new_open_open(1.0, 1.0).is_empty());
        assert!(Interval::new_open_closed(1.0, 1.0).is_empty());

        // In machine representation, nothing between 1.0 and one_eps
        let one_eps = 1.0 + f32::EPSILON;
        assert!(!Interval::new_closed_closed(1.0, one_eps).is_empty());
        assert!(!Interval::new_closed_open(1.0, one_eps).is_empty());
        assert!(Interval::new_open_open(1.0, one_eps).is_empty());
        assert!(!Interval::new_open_open(1.0, 2.0 + one_eps).is_empty());
        assert!(!Interval::new_open_closed(1.0, one_eps).is_empty());

        // Empty since left bound is greater than right bound
        let one_min_eps = 1.0 - f32::EPSILON;
        assert!(Interval::new_closed_closed(1.0, one_min_eps).is_empty());
        assert!(Interval::new_closed_open(1.0, one_min_eps).is_empty());
        assert!(Interval::new_open_closed(1.0, one_min_eps).is_empty());
        assert!(Interval::new_open_open(1.0, one_min_eps).is_empty());

        // In mathematical representation, an infinite number of reals between
        // 1.0 and one_eps
        let real_1 = Mathf32(1.0);
        let real_1_eps = Mathf32(1.0 + f32::EPSILON);
        assert!(!Interval::new_closed_closed(real_1, real_1_eps).is_empty());
        assert!(!Interval::new_closed_open(real_1, real_1_eps).is_empty());
        assert!(!Interval::new_open_closed(real_1, real_1_eps).is_empty());
        assert!(!Interval::new_open_open(real_1, real_1_eps).is_empty());

        // When the bounds cannot be compared, the interval is empty
        assert!(Interval::new_closed_open(1.0, f32::NAN).is_empty());
        assert!(Interval::new_closed_closed(1.0, f32::NAN).is_empty());
        assert!(Interval::new_open_closed(1.0, f32::NAN).is_empty());
        assert!(Interval::new_open_open(1.0, f32::NAN).is_empty());
        assert!(Interval::new_closed_open(f32::NAN, 1.0).is_empty());
        assert!(Interval::new_closed_closed(f32::NAN, 1.0).is_empty());
        assert!(Interval::new_open_closed(f32::NAN, 1.0).is_empty());
        assert!(Interval::new_open_open(f32::NAN, 1.0).is_empty());

        assert!(!Interval::new_unbounded_closed(5.0).is_empty());
        assert!(!Interval::new_unbounded_open(5.0).is_empty());
        assert!(!Interval::new_closed_unbounded(5.0).is_empty());
        assert!(!Interval::new_open_unbounded(5.0).is_empty());
        assert!(!Interval::<u32>::doubly_unbounded().is_empty());

        // Test NothingBetween for standard types
        assert!(Interval::new_closed_open(1_u8, 1).is_empty());
        assert!(Interval::new_open_open(0_u8, 1).is_empty());
        assert!(Interval::new_open_open(2_u8, 1).is_empty());

        assert!(Interval::new_closed_open(1_u16, 1).is_empty());
        assert!(Interval::new_open_open(0_u16, 1).is_empty());
        assert!(Interval::new_open_open(2_u16, 1).is_empty());

        assert!(Interval::new_closed_open(1_u32, 1).is_empty());
        assert!(Interval::new_open_open(0_u32, 1).is_empty());
        assert!(Interval::new_open_open(2_u32, 1).is_empty());

        assert!(Interval::new_closed_open(1_u64, 1).is_empty());
        assert!(Interval::new_open_open(0_u64, 1).is_empty());
        assert!(Interval::new_open_open(2_u64, 1).is_empty());

        assert!(Interval::new_closed_open(1_i8, 1).is_empty());
        assert!(Interval::new_open_open(0_i8, 1).is_empty());
        assert!(Interval::new_open_open(2_i8, 1).is_empty());

        assert!(Interval::new_closed_open(1_i16, 1).is_empty());
        assert!(Interval::new_open_open(0_i16, 1).is_empty());
        assert!(Interval::new_open_open(2_i16, 1).is_empty());

        assert!(Interval::new_closed_open(1_i32, 1).is_empty());
        assert!(Interval::new_open_open(0_i32, 1).is_empty());
        assert!(Interval::new_open_open(2_i32, 1).is_empty());

        assert!(Interval::new_closed_open(1_i64, 1).is_empty());
        assert!(Interval::new_open_open(0_i64, 1).is_empty());
        assert!(Interval::new_open_open(2_i64, 1).is_empty());

        assert!(Interval::new_closed_open(1.0_f32, 1.0).is_empty());
        assert!(!Interval::new_open_open(0.0_f32, 1.0).is_empty());
        assert!(Interval::new_open_open(2.0_f32, 1.0).is_empty());

        assert!(Interval::new_closed_open(1.0_f64, 1.0).is_empty());
        assert!(!Interval::new_open_open(0.0_f64, 1.0).is_empty());
        assert!(Interval::new_open_open(2.0_f64, 1.0).is_empty());

        assert!(Interval::new_closed_open('b', 'b').is_empty());
        assert!(Interval::new_open_open('a', 'b').is_empty());
        assert!(Interval::new_open_open('c', 'b').is_empty());

        assert!(Interval::new_closed_open(&1_u64, &1).is_empty());
        assert!(Interval::new_open_open(&0_u64, &1).is_empty());
        assert!(Interval::new_open_open(&2_u64, &1).is_empty());

    }

    #[test]
    fn test_single() {
        let intv = Interval::new_single(4);
        assert!(!intv.is_empty());
        assert!(intv.is_single());
        assert!(intv.contains(&4));
        assert!(!intv.contains(&5));

        let intv = Interval::new_single(f32::NAN);
        assert!(intv.is_empty());
        assert!(!intv.is_single());

        assert!(!Interval::new_closed_open(1, 4).is_single());
        assert!(Interval::new_closed_closed(1, 1).is_single());
        assert!(Interval::new_closed_closed(1.0, 1.0).is_single());

        // An interval that contains a single element, but is not of the form
        // [A,A] will return false for is_single
        assert!(!Interval::new_open_open(0, 2).is_single());
    }

    #[test]
    fn test_equivalent() {
        let intv1 = Interval::new_closed_open(1, 4);
        let intv2 = Interval::new_closed_closed(1, 3);
        let intv4 = Interval::new_open_closed(0, 3);
        let intv5 = Interval::new_open_open(0, 4);
        let intv6 = Interval::new_open_open(-1, 3);
        let intv7 = Interval::new_closed_closed(1, 5);
        assert_equivalent(&intv1, &intv1);
        assert_equivalent(&intv1, &intv2);
        assert_equivalent(&intv1, &intv4);
        assert_equivalent(&intv1, &intv5);
        assert_equivalent(&intv5, &intv2);
        assert_not_equivalent(&intv1, &intv7);
        assert_not_equivalent(&intv5, &intv6);

        let intv3 = Interval::new_closed_closed(1, 4);
        assert_not_equivalent(&intv1, &intv3);
        assert_not_equivalent(&intv2, &intv3);

        // Note: this will fail when using larger values than 1.0, because
        // f32 cannot distinguish between 4.0 and 4.0 - EPSILON for instance.
        // But that would be user-error, not an issue with intervals.
        let f1 = Interval::new_closed_open(0.0, 1.0);
        let f2 = Interval::new_closed_closed(0.0, 1.0);
        assert_not_equivalent(&f1, &f2);
        let f3 = Interval::new_closed_closed(0.0, 1.0 - f32::EPSILON);
        assert_equivalent(&f1, &f3);

        let r1 = Interval::new_closed_open(Mathf32(0.0), Mathf32(1.0));
        let r2 = Interval::new_closed_closed(Mathf32(0.0), Mathf32(1.0));
        assert_not_equivalent(&r1, &r2);
        let r3 = Interval::new_closed_closed(
            Mathf32(0.0),
            Mathf32(1.0 - f32::EPSILON),
        );
        assert_not_equivalent(&r1, &r3);

        let u1 = Interval::new_unbounded_open(10);
        let u2 = Interval::new_unbounded_closed(9);
        assert_equivalent(&u1, &u2);
        assert_not_equivalent(&u1, &intv1);

        let u1 = Interval::new_open_unbounded(9);
        let u2 = Interval::new_closed_unbounded(10);
        assert_equivalent(&u1, &u2);
        assert_not_equivalent(&u1, &intv1);

        let empty = Interval::default();
        assert_equivalent(&empty, &empty);
        assert_not_equivalent(&empty, &intv1);
    }

    #[test]
    fn test_io() {
        assert_eq!(format!("{}", Interval::new_closed_closed(1, 4)), "[1, 4]",);
        assert_eq!(format!("{}", Interval::new_closed_open(1, 4)), "[1, 4)",);
        assert_eq!(format!("{}", Interval::new_open_closed(1, 4)), "(1, 4]",);
        assert_eq!(format!("{}", Interval::new_open_open(1, 4)), "(1, 4)",);
        assert_eq!(format!("{}", Interval::new_closed_unbounded(1)), "[1,)",);
        assert_eq!(format!("{}", Interval::new_open_unbounded(1)), "(1,)",);
        assert_eq!(format!("{}", Interval::new_unbounded_closed(1)), "(, 1]",);
        assert_eq!(format!("{}", Interval::new_unbounded_open(1)), "(, 1)",);
        assert_eq!(format!("{}", Interval::<f32>::doubly_unbounded()), "(,)",);
        assert_eq!(format!("{}", Interval::<f32>::empty()), "empty",);
        assert_eq!(
            format!("{}", Interval::new_closed_closed(1.0_f32, 4.0 - 0.1)),
            "[1, 3.9]",
        );
        assert_eq!(
            format!("{}", Interval::new_closed_closed(1.0, 4.0 - f32::EPSILON)),
            "[1, 4]",
        );
        assert_eq!(
            format!(
                "{:?}",
                Interval::new_closed_closed(1.0, 4.0 - f32::EPSILON)
            ),
            "(LeftOf(1.0),RightOf(4.0))",
        );
        assert_eq!(format!("{:?}", Interval::<f32>::empty()), "empty");
        assert_eq!(
            format!("{:?}", Interval::<f32>::doubly_unbounded()),
            "(-infinity,+infinity)"
        );
    }

    #[test]
    fn test_ord() {
        let b1 = Bound::LeftOf(3); //  2 < b1 < 3 < b2 < 4
        let b2 = Bound::RightOf(3);
        assert!(b1 != b2);
        assert!(b1 < b2);

        let b3 = Bound::LeftOf(4);
        assert!(b3 == b2);
        assert!(b2 == b3);
    }

    #[test]
    fn test_left_of() {
        let intv1 = Interval::new_closed_open(3_i8, 5); // [3,5)
        assert!(intv1.strictly_left_of(&6));
        assert!(intv1.strictly_left_of(&5));
        assert!(!intv1.strictly_left_of(&0));
        assert!(!intv1.strictly_left_of(&3));

        assert!(intv1.left_of(&6));
        assert!(intv1.left_of(&5));
        assert!(!intv1.left_of(&0));
        assert!(!intv1.left_of(&3));

        assert!(intv1.strictly_right_of(&0));
        assert!(intv1.strictly_right_of(&2));
        assert!(!intv1.strictly_right_of(&3));

        assert!(intv1.right_of(&0));
        assert!(intv1.right_of(&2));
        assert!(intv1.right_of(&3));

        let intv2 = Interval::new_closed_closed(3, 5);
        assert!(intv2.left_of(&6));
        assert!(intv2.left_of(&5));
        assert!(!intv2.strictly_left_of(&5));

        assert!(!intv1.strictly_left_of_interval(&intv2));
        assert!(!intv2.strictly_left_of_interval(&intv1));

        let empty = Interval::<i8>::empty();
        assert!(empty.strictly_left_of(&1));
        assert!(empty.left_of(&1));
        assert!(empty.strictly_right_of(&1));
        assert!(empty.right_of(&1));
        assert!(empty.strictly_left_of_interval(&intv1));
        assert!(intv1.strictly_left_of_interval(&empty));

        let intv6 = Interval::new_open_closed(3, 5); // (3,5]
        let intv3 = Interval::new_closed_closed(1, 3); // [1,3]
        assert!(!intv3.strictly_left_of_interval(&intv1));
        assert!(!intv1.strictly_left_of_interval(&intv3));
        assert!(intv3.strictly_left_of_interval(&intv6));
        assert!(!intv6.strictly_left_of_interval(&intv3));

        let intv4 = Interval::new_closed_closed(0, 1);
        assert!(intv4.strictly_left_of_interval(&intv1));
        assert!(!intv1.strictly_left_of_interval(&intv4));

        let intv5 = Interval::new_closed_unbounded(1); // [1,)
        assert!(!intv5.strictly_left_of_interval(&intv1));
        assert!(!intv5.right_of(&10));
        assert!(intv5.strictly_right_of(&0));
        assert!(intv5.right_of(&0));

        let intv7 = Interval::new_unbounded_closed(10_i16);
        assert!(!intv7.right_of(&0));
        assert!(!intv7.strictly_right_of(&0));
    }

    #[test]
    fn test_ref() {
        let intv1 = Interval::<&char>::new_closed_closed(&'A', &'Z');
        assert!(!intv1.is_empty());
        assert!(intv1.contains(&&'B'));
        assert!(!intv1.contains(&&'a'));

        let intv2 = Interval::<char>::new_closed_closed('A', 'Z');
        assert!(intv2.as_ref().contains_interval(&intv1));
    }

    #[test]
    fn test_convex_hull() {
        let intv1 = Interval::new_closed_closed(10, 30);
        let intv2 = Interval::new_closed_closed(40, 50);
        assert_eq!(
            intv1.convex_hull(&intv2),
            Interval::new_closed_closed(10, 50)
        );
        assert_eq!(
            intv2.convex_hull(&intv1),
            Interval::new_closed_closed(10, 50)
        );

        let intv1 = Interval::new_closed_closed(10, 30);
        let intv2 = Interval::new_closed_closed(20, 30); // nested
        assert_eq!(intv1.convex_hull(&intv2), intv1);
        assert_eq!(intv2.convex_hull(&intv1), intv1);
        assert_eq!(intv2.union(&intv1), Some(intv1));

        let intv1 = Interval::new_open_open(10, 30);
        let intv2 = Interval::new_open_open(40, 50); // nested
        assert_eq!(intv1.convex_hull(&intv2), Interval::new_open_open(10, 50));
        assert_eq!(intv2.convex_hull(&intv1), Interval::new_open_open(10, 50));
        assert_eq!(intv2.union(&intv1), None); //  not contiguous

        let intv1 = Interval::empty();
        let intv2 = Interval::new_open_open(40, 50); // nested
        assert_eq!(intv1.convex_hull(&intv2), intv2);
        assert_eq!(intv2.convex_hull(&intv1), intv2);
        assert_eq!(intv2.union(&intv1), Some(intv2));

        let intv1 = Interval::new_open_unbounded(10);
        let intv2 = Interval::new_open_open(40, 50); // nested
        assert_eq!(intv1.convex_hull(&intv2), intv1);
        assert_eq!(intv2.convex_hull(&intv1), intv1);
        assert_eq!(intv2.union(&intv1), Some(intv1));

        let intv1 = Interval::new_unbounded_open(10);
        let intv2 = Interval::new_open_open(40, 50); // nested
        assert_eq!(intv1.convex_hull(&intv2), Interval::new_unbounded_open(50));
        assert_eq!(intv2.convex_hull(&intv1), Interval::new_unbounded_open(50));
        assert_eq!(intv2.union(&intv1), None);
    }

    #[test]
    fn test_difference() {
        let intv1 = Interval::new_closed_closed(10, 30);
        let empty = Interval::<i32>::empty();
        assert_eq!(intv1.difference(&empty), MultiInterval::One(intv1.clone()));
        assert_eq!(empty.difference(&intv1), MultiInterval::One(empty.clone()));

        let intv2 = Interval::new_closed_closed(1, 50); //  larger
        assert_eq!(intv1.difference(&intv2), MultiInterval::One(empty.clone()));
        assert_eq!(
            intv2.difference(&intv1),
            MultiInterval::Two(
                Interval::new_closed_open(1, 10),
                Interval::new_open_closed(30, 50),
            )
        );
        assert_eq!(
            format!("{:?}", intv2.difference(&intv1)),
            "((LeftOf(1),LeftOf(10)) + (RightOf(30),RightOf(50)))"
        );

        let intv3 = Interval::new_closed_closed(1, 5); // disjoint
        assert_eq!(intv1.difference(&intv3), MultiInterval::One(intv1.clone()));
        assert_eq!(intv3.difference(&intv1), MultiInterval::One(intv3.clone()));
        assert_eq!(
            format!("{:?}", intv1.difference(&intv3)),
            "(LeftOf(10),RightOf(30))"
        );

        let intv4 = Interval::new_closed_closed(1, 15); // overlaps left
        assert_eq!(
            intv1.difference(&intv4),
            MultiInterval::One(Interval::new_open_closed(15, 30))
        );

        let intv5 = Interval::new_closed_closed(25, 40); // overlaps right
        assert_eq!(
            intv1.difference(&intv5),
            MultiInterval::One(Interval::new_closed_open(10, 25))
        );

        //  Check the variants of subtraction
        assert_eq!(&intv1 - &empty, MultiInterval::One(intv1.clone()));
        let e = empty.clone();
        assert_eq!(&intv1 - e, MultiInterval::One(intv1.clone()));
        let i = intv1.clone();
        assert_eq!(i - &empty, MultiInterval::One(intv1.clone()));
        let i = intv1.clone();
        let e = empty.clone();
        assert_eq!(i - e, MultiInterval::One(intv1.clone()));
    }

    #[test]
    fn test_unusual_bounds() {
        // We can actually declare intervals for types that we can't even
        // compare, although a lot of the functions are not available
        let intv1 = Interval::new_closed_open("abc", "def");
        assert_eq!(intv1.lower(), Some(&"abc"));
        assert!(intv1.lower_inclusive());
        assert!(!intv1.lower_unbounded());
        assert_eq!(intv1.upper(), Some(&"def"));
        assert!(!intv1.upper_inclusive());
        assert!(!intv1.upper_unbounded());

        let intv2 = Interval::new_closed_unbounded("abc");
        assert_eq!(intv2.lower(), Some(&"abc"));
        assert!(intv2.lower_inclusive());
        assert!(!intv2.lower_unbounded());
        assert_eq!(intv2.upper(), None);
        assert!(!intv2.upper_inclusive());
        assert!(intv2.upper_unbounded());

        let intv3 =
            Interval::new_closed_open("abc".to_string(), "def".to_string());
        let _intv4 = intv3.as_ref();

        let intv5 = Interval::new_closed_open('a', 'c');
        assert!(!intv5.is_empty());

        // With references
        let intv5 = Interval::new_closed_open(&'a', &'c');
        assert!(!intv5.is_empty());
    }

    #[test]
    fn test_between() {
        let intv1 = Interval::new_closed_closed(10, 30);
        let intv2 = Interval::new_closed_closed(40, 50);
        let intv3 = Interval::new_open_unbounded(35);
        let empty = Interval::empty();
        assert_eq!(intv1.between(&intv2), Interval::new_open_open(30, 40),);
        assert_eq!(intv1.between(&intv3), Interval::new_open_closed(30, 35),);
        assert_eq!(intv2.between(&intv3), empty.clone(),);
        assert_eq!(intv1.between(&empty), empty.clone(),);
        assert_eq!(empty.between(&intv1), empty.clone(),);
        assert!(intv1.contiguous(&intv1));
        assert!(!intv1.contiguous(&intv2));
        assert!(!intv1.contiguous(&intv3));
        assert!(intv2.contiguous(&intv3));
        assert!(empty.contiguous(&intv1));
        assert!(intv1.contiguous(&empty));
    }

    #[test]
    fn test_intersection() {
        let intv1 = Interval::new_closed_closed(10_u8, 30);
        let intv2 = Interval::new_closed_open(40_u8, 50);
        let intv3 = Interval::new_open_unbounded(35_u8);
        let empty = Interval::empty();
        assert!(!intv1.intersects(&intv2));
        assert_eq!(intv1.intersection(&intv2), empty.clone());
        assert!(intv2.intersects(&intv3));
        assert_eq!(
            intv2.intersection(&intv3),
            Interval::new_closed_open(40, 50)
        );

        //  Check the variants of "&"
        assert_eq!(&intv1 & &intv2, empty.clone());
        let iv2 = intv2.clone();
        assert_eq!(&intv1 & iv2, empty.clone());
        let iv1 = intv1.clone();
        let iv2 = intv2.clone();
        assert_eq!(iv1 & &iv2, empty.clone());
        let iv1 = intv1.clone();
        assert_eq!(iv1 & iv2, empty.clone());
    }

    #[test]
    fn test_symmetric_difference() {
        let intv1 = Interval::new_closed_closed(10, 30);
        let empty = Interval::<i32>::empty();
        assert_eq!(
            intv1.symmetric_difference(&empty),
            MultiInterval::One(intv1.clone())
        );
        assert_eq!(
            empty.symmetric_difference(&intv1),
            MultiInterval::One(intv1.clone())
        );

        let intv2 = Interval::new_closed_closed(1, 50); //  larger
        assert_eq!(
            intv1.symmetric_difference(&intv2),
            MultiInterval::Two(
                Interval::new_closed_open(1, 10),
                Interval::new_open_closed(30, 50),
            ),
        );
        assert_eq!(
            intv2.symmetric_difference(&intv1),
            MultiInterval::Two(
                Interval::new_closed_open(1, 10),
                Interval::new_open_closed(30, 50),
            )
        );

        let intv3 = Interval::new_closed_closed(1, 5); // disjoint
        assert_eq!(
            intv1.symmetric_difference(&intv3),
            MultiInterval::Two(intv3.clone(), intv1.clone(),),
        );
        assert_eq!(
            intv3.symmetric_difference(&intv1),
            MultiInterval::Two(intv3.clone(), intv1.clone(),),
        );

        let intv4 = Interval::new_closed_closed(1, 15); // overlaps left
        assert_eq!(
            intv1.symmetric_difference(&intv4),
            MultiInterval::Two(
                Interval::new_closed_open(1, 10),
                Interval::new_open_closed(15, 30),
            ),
        );

        let intv5 = Interval::new_closed_closed(25, 40); // overlaps right
        assert_eq!(
            intv1.symmetric_difference(&intv5),
            MultiInterval::Two(
                Interval::new_closed_open(10, 25),
                Interval::new_open_closed(30, 40),
            ),
        );

        //  Check the variants of subtraction
        assert_eq!(&intv1 ^ &empty, MultiInterval::One(intv1.clone()));
        let e = empty.clone();
        assert_eq!(&intv1 ^ e, MultiInterval::One(intv1.clone()));
        let i = intv1.clone();
        assert_eq!(i ^ &empty, MultiInterval::One(intv1.clone()));
        let i = intv1.clone();
        let e = empty.clone();
        assert_eq!(i ^ e, MultiInterval::One(intv1.clone()));
    }
}
