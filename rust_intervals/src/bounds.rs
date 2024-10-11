use crate::nothing_between::NothingBetween;
use std::cmp::{Ordering, PartialOrd};

/// One bound of an interval
/// LeftOf, applied to value, represents a conceptual point halfway between
/// the value and its predecessor value.
/// Likewise, RightOf represents a conceptual point halfway between the value
/// and its successor.
pub(crate) enum Bound<T> {
    LeftUnbounded,
    LeftOf(T),
    RightOf(T),
    RightUnbounded,
}

impl<T: PartialOrd> Bound<T> {
    /// True if value is to the right of the bound
    pub(crate) fn left_of(&self, value: &T) -> bool {
        match self {
            Bound::LeftUnbounded => true,
            Bound::LeftOf(point) => *point <= *value,
            Bound::RightOf(point) => *point < *value,
            Bound::RightUnbounded => false,
        }
    }

    /// True if the value is to the left of the bound
    pub(crate) fn right_of(&self, value: &T) -> bool {
        match self {
            Bound::LeftUnbounded => false,
            Bound::LeftOf(point) => *value < *point,
            Bound::RightOf(point) => *value <= *point,
            Bound::RightUnbounded => true,
        }
    }
}

impl<T: PartialOrd + NothingBetween + Clone> Bound<T> {
    pub(crate) fn min(&self, right: &Self) -> Self {
        if self < right {
            self.clone()
        } else {
            right.clone()
        }
    }

    pub(crate) fn max(&self, right: &Self) -> Self {
        if self > right {
            self.clone()
        } else {
            right.clone()
        }
    }
}

impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Bound<T> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            Bound::LeftUnbounded => write!(f, "-infinity")?,
            Bound::LeftOf(point) => write!(f, "LeftOf({point:?})")?,
            Bound::RightOf(point) => write!(f, "RightOf({point:?})")?,
            Bound::RightUnbounded => write!(f, "+infinity")?,
        }
        Ok(())
    }
}

impl<T> Bound<T> {
    /// Return the bound's value (which might be included in the interval
    /// or not).  This returns None for an unbounded bound.
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Bound::LeftUnbounded | Bound::RightUnbounded => None,
            Bound::LeftOf(p) | Bound::RightOf(p) => Some(p),
        }
    }

    /// Converts from `Bound<T>` to `Bound<&T>`
    pub(crate) fn as_ref(&self) -> Bound<&T> {
        match self {
            Bound::LeftUnbounded => Bound::LeftUnbounded,
            Bound::LeftOf(point) => Bound::LeftOf(point),
            Bound::RightOf(point) => Bound::RightOf(point),
            Bound::RightUnbounded => Bound::RightUnbounded,
        }
    }
}

impl<T: PartialOrd + NothingBetween> PartialEq<Bound<&T>> for Bound<T> {
    //  Bound is never equal to an exact value.  Doesn't matter since we only
    //  compare for strict inequality
    fn eq(&self, other: &Bound<&T>) -> bool {
        match (self, other) {
            (Bound::LeftUnbounded, Bound::LeftUnbounded)
            | (Bound::RightUnbounded, Bound::RightUnbounded) => true,
            (Bound::LeftOf(s), Bound::LeftOf(o))
            | (Bound::RightOf(s), Bound::RightOf(o)) => *s == **o,
            (Bound::LeftOf(s), Bound::RightOf(o)) => match s.partial_cmp(o) {
                None | Some(Ordering::Less | Ordering::Equal) => false,
                Some(Ordering::Greater) => (*o).nothing_between(s),
            },
            (Bound::RightOf(s), Bound::LeftOf(o)) => match s.partial_cmp(o) {
                None | Some(Ordering::Equal | Ordering::Greater) => false,
                Some(Ordering::Less) => s.nothing_between(o),
            },
            (Bound::LeftUnbounded, _)
            | (_, Bound::LeftUnbounded)
            | (_, Bound::RightUnbounded)
            | (Bound::RightUnbounded, _) => false,
        }
    }
}

impl<T: PartialOrd + NothingBetween> PartialEq for Bound<T> {
    //  Bound is never equal to an exact value.  Doesn't matter since we only
    //  compare for strict inequality
    fn eq(&self, other: &Bound<T>) -> bool {
        self.eq(&other.as_ref())
    }
}

impl<T: PartialOrd + NothingBetween> PartialOrd for Bound<T> {
    fn partial_cmp(&self, other: &Bound<T>) -> Option<Ordering> {
        self.partial_cmp(&other.as_ref())
    }
}

impl<T: PartialOrd + NothingBetween> PartialOrd<Bound<&T>> for Bound<T> {
    /// Two bounds (either lower and upper of same interval, or possibly
    /// lowers from two intervals) might be equivalent if there is nothing
    /// between them.
    /// For instance, for f32:
    ///     RightOf(1.0) is equivalent to LeftOf(1.0 + EPSILON)
    ///     since there is nothing between 1.0 and 1.0 + EPSILON
    /// (this would not be true when talking about mathematical reals for
    /// instance).
    /// This function returns Equal if there is nothing between the two
    /// bounds.
    fn partial_cmp(&self, other: &Bound<&T>) -> Option<Ordering> {
        match (self, other) {
            (Bound::LeftUnbounded, Bound::LeftUnbounded)
            | (Bound::RightUnbounded, Bound::RightUnbounded) => {
                Some(Ordering::Equal)
            }
            (Bound::LeftOf(s), Bound::LeftOf(o))
            | (Bound::RightOf(s), Bound::RightOf(o)) => s.partial_cmp(*o),
            (Bound::LeftOf(s), Bound::RightOf(o)) => match s.partial_cmp(o) {
                None => None,
                Some(Ordering::Less | Ordering::Equal) => Some(Ordering::Less),
                Some(Ordering::Greater) => Some(if (*o).nothing_between(s) {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }),
            },
            (Bound::RightOf(s), Bound::LeftOf(o)) => match s.partial_cmp(*o) {
                None => None,
                Some(Ordering::Less) => Some(if s.nothing_between(*o) {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }),
                Some(Ordering::Equal | Ordering::Greater) => {
                    Some(Ordering::Greater)
                }
            },
            (Bound::LeftUnbounded, _) => Some(Ordering::Less),
            (_, Bound::LeftUnbounded) => Some(Ordering::Greater),
            (_, Bound::RightUnbounded) => Some(Ordering::Less),
            (Bound::RightUnbounded, _) => Some(Ordering::Greater),
        }
    }
}

impl<T: Clone> std::clone::Clone for Bound<T> {
    fn clone(&self) -> Self {
        match self {
            Bound::LeftUnbounded => Bound::LeftUnbounded,
            Bound::RightUnbounded => Bound::RightUnbounded,
            Bound::LeftOf(point) => Bound::LeftOf(point.clone()),
            Bound::RightOf(point) => Bound::RightOf(point.clone()),
        }
    }
}
