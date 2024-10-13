use std::cmp::{Ordering, PartialOrd};
use crate::bounds::Bound;
use crate::multi_intervals::MultiInterval;
use crate::nothing_between::NothingBetween;

/// An interval of values.
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
