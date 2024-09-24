use chrono::{DateTime, TimeZone};
use std::cmp::{Ordering, PartialOrd};

/// An interval of values like:
///    [A, B]    left-closed, right-closed
///    [A, B)    left-closed, right-open
///    (A, B)    left-open, right-open
///    (A, B]    left-open, right-closed
pub struct BoundedInterval<T> {
    lower: Bound<T>,
    upper: Bound<T>,
}

impl<T: PartialOrd + NothingBetween> BoundedInterval<T> {
    /// Left-closed, Right-open intervals
    ///    [A, B)
    pub fn lcro(lower: T, upper: T) -> Self {
        Self {
            lower: Bound {
                point: lower,
                offset: OffsetSide::Left,
            },
            upper: Bound {
                point: upper,
                offset: OffsetSide::Left,
            },
        }
    }

    /// Left-closed, Right-closed intervals
    ///    [A, B]
    pub fn lcrc(lower: T, upper: T) -> Self {
        Self {
            lower: Bound {
                point: lower,
                offset: OffsetSide::Left,
            },
            upper: Bound {
                point: upper,
                offset: OffsetSide::Right,
            },
        }
    }

    /// Whether the value is contained in the interval
    pub fn contains(&self, value: &T) -> bool {
        self.lower.left_of(value) && self.upper.right_of(value)
    }
    pub fn contains_interval(&self, other: &Self) -> bool {
        other.is_empty()
            || (self.lower <= other.lower && other.upper <= self.upper)
    }

    /// True if the range contains no element
    pub fn is_empty(&self) -> bool {
        self.upper <= self.lower
    }

    /// Whether the two ranges contain the same set of values
    pub fn equivalent(&self, other: &Self) -> bool {
        if self.is_empty() {
            other.is_empty()
        } else if other.is_empty() {
            false
        } else {
            self.lower == other.lower && self.upper == other.upper
        }
    }

    /// Whether every value in the range is strictly less than (<) X.  (True is
    /// returned if R is empty).
    pub fn strictly_left_of(&self, x: &T) -> bool {
        self.is_empty() || self.upper.left_of(x)
    }

    /// Whether X is strictly less than (<) every value in the range.  (True is
    /// returned if R is empty).
    pub fn strictly_right_of(&self, x: &T) -> bool {
        self.is_empty() || self.lower.right_of(x)
    }
}

impl<T: PartialOrd + Clone + NothingBetween> BoundedInterval<T> {
    /// Whether every value in the range is less than (<=) X.  (True is returned
    /// if R is empty).
    pub fn left_of(&self, x: &T) -> bool {
        self.is_empty()
            || self.upper
                <= Bound {
                    point: x.clone(),
                    offset: OffsetSide::Right,
                }
    }

    /// Whether X is less than (<=) every value in the range.  (True is returned
    /// if R is empty).
    pub fn right_of(&self, x: &T) -> bool {
        self.is_empty()
            || Bound {
                point: x.clone(),
                offset: OffsetSide::Left,
            } <= self.lower
    }
}

impl<T: Default> BoundedInterval<T> {
    /// Returns an empty range
    pub fn new_empty_range() -> Self {
        Self {
            lower: Bound {
                point: T::default(),
                offset: OffsetSide::Right,
            },
            upper: Bound {
                point: T::default(),
                offset: OffsetSide::Left,
            },
        }
    }
}

impl<T: Clone> BoundedInterval<T> {
    /// Returns a range that contains a single value
    pub fn new_single(value: T) -> Self {
        Self {
            lower: Bound {
                point: value.clone(),
                offset: OffsetSide::Left,
            },
            upper: Bound {
                point: value,
                offset: OffsetSide::Right,
            },
        }
    }

    /// The lower bound
    pub fn lower(&self) -> T {
        self.lower.point.clone()
    }

    /// Whether the lower bound is part of the interval
    pub fn lower_inclusive(&self) -> bool {
        matches!(self.lower.offset, OffsetSide::Left)
    }

    /// The upper bound
    pub fn upper(&self) -> T {
        self.upper.point.clone()
    }

    /// Whether the upper bound is part of the interval
    pub fn upper_inclusive(&self) -> bool {
        matches!(self.upper.offset, OffsetSide::Right)
    }
}

impl<T: Clone> std::clone::Clone for BoundedInterval<T> {
    fn clone(&self) -> Self {
        Self {
            lower: self.lower.clone(),
            upper: self.upper.clone(),
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for BoundedInterval<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.lower.offset {
            OffsetSide::Left => write!(f, "[")?,
            OffsetSide::Right => write!(f, "(")?,
        }
        write!(f, "{}, {}", self.lower.point, self.upper.point)?;
        match self.upper.offset {
            OffsetSide::Left => write!(f, ")")?,
            OffsetSide::Right => write!(f, "]")?,
        }
        Ok(())
    }
}

pub trait NothingBetween {
    fn nothing_between(&self, other: &Self) -> bool;
    //  Should return True if no value exists between self and other in this
    //  type.
    //  This is only called with self < other.
}

impl NothingBetween for u8 {
    fn nothing_between(&self, other: &u8) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for u16 {
    fn nothing_between(&self, other: &u16) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for u32 {
    fn nothing_between(&self, other: &u32) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for u64 {
    fn nothing_between(&self, other: &u64) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for i8 {
    fn nothing_between(&self, other: &i8) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for i16 {
    fn nothing_between(&self, other: &i16) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for i32 {
    fn nothing_between(&self, other: &i32) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for i64 {
    fn nothing_between(&self, other: &i64) -> bool {
        other - self <= 1
    }
}
impl NothingBetween for f32 {
    fn nothing_between(&self, _other: &f32) -> bool {
        false
        // In the world of real, there is always something in-between, even if
        // we cannot represent it.  However, in this case we may have a range
        // for which is_empty() return false, but which actually contain no
        // values, e.g.  (A, A + f32::EPSILON)
    }
}
impl NothingBetween for f64 {
    fn nothing_between(&self, _other: &f64) -> bool {
        false
    }
}
impl<T: TimeZone> NothingBetween for DateTime<T> {
    fn nothing_between(&self, _other: &DateTime<T>) -> bool {
        false
    }
}

/// Left, applied to value, represents a conceptual point halfway between
/// the value and its predecessor value.
/// Likewise, Right represents a conceptual point halfway between the value
/// and its successor.
#[derive(Clone, Copy, Eq, PartialEq)]
enum OffsetSide {
    Left,
    Right,
}

/// One bound of an interval
struct Bound<T> {
    point: T,
    offset: OffsetSide,
}

impl<T: PartialOrd> Bound<T> {
    /// True if the value is to the right of the bound
    fn left_of(&self, value: &T) -> bool {
        match self.offset {
            OffsetSide::Left => self.point <= *value,
            OffsetSide::Right => self.point < *value,
        }
    }

    /// True if the value is to the left of the bound
    fn right_of(&self, value: &T) -> bool {
        match self.offset {
            OffsetSide::Left => *value < self.point,
            OffsetSide::Right => *value <= self.point,
        }
    }
}

impl<T: PartialOrd + NothingBetween> PartialEq for Bound<T> {
    //  Bound is never equal to an exact value.  Doesn't matter since we only
    //  compare for strict inequality
    fn eq(&self, other: &Bound<T>) -> bool {
        !(self < other || other < self)
    }
}

impl<T: PartialOrd + NothingBetween> PartialOrd for Bound<T> {
    fn partial_cmp(&self, other: &Bound<T>) -> Option<Ordering> {
        if self.offset == other.offset {
            return if self.point < other.point {
                Some(Ordering::Less)
            } else if self.point == other.point {
                Some(Ordering::Equal)
            } else {
                Some(Ordering::Greater)
            };
        }
        if self.offset == OffsetSide::Left {
            //  then other is Right
            return Some(if self.point <= other.point {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        Some(
            if self.point < other.point
                && !self.point.nothing_between(&other.point)
            {
                Ordering::Less
            } else {
                Ordering::Greater
            },
        )
    }
}

impl<T: Clone> std::clone::Clone for Bound<T> {
    fn clone(&self) -> Self {
        Self {
            point: self.point.clone(),
            offset: self.offset,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_contains() {
        let intv = BoundedInterval::lcro(1, 10);
        assert!(intv.contains(&1));
        assert!(intv.contains(&2));
        assert!(intv.contains(&9));
        assert!(!intv.contains(&10));

        let intv2 = BoundedInterval::lcrc(1, 5);
        assert!(intv2.contains(&1));
        assert!(intv2.contains(&5));

        assert!(intv.contains_interval(&intv2));
        assert!(!intv2.contains_interval(&intv));
    }

    #[test]
    fn test_inclusive() {
        let intv = BoundedInterval::lcro(1, 10);
        assert_eq!(intv.lower(), 1);
        assert!(intv.lower_inclusive());
        assert_eq!(intv.upper(), 10);
        assert!(!intv.upper_inclusive());

        let intv = BoundedInterval::lcrc(1, 10);
        assert_eq!(intv.lower(), 1);
        assert!(intv.lower_inclusive());
        assert_eq!(intv.upper(), 10);
        assert!(intv.upper_inclusive());
    }

    #[test]
    fn test_empty() {
        let intv = BoundedInterval::lcro(1, 10);
        assert!(!intv.is_empty());

        let intv = BoundedInterval::lcro(1, 1);
        assert!(intv.is_empty());

        let intv = BoundedInterval::lcro(1, 0);
        assert!(intv.is_empty());

        let empty = BoundedInterval::<f32>::new_empty_range();
        assert!(empty.is_empty());
        assert!(!empty.contains(&1.1));

        let intv = BoundedInterval::lcro(1.0, 1.0);
        assert!(intv.is_empty());
        let intv = BoundedInterval::lcrc(1.0, 1.0);
        assert!(!intv.is_empty());
        let intv = BoundedInterval::lcrc(1.0, 1.0 - f32::EPSILON);
        assert!(intv.is_empty());
        let intv = BoundedInterval::lcrc(1.0, 1.0 + f32::EPSILON);
        assert!(!intv.is_empty());
        let intv = BoundedInterval::lcro(1.0, 1.0 + f32::EPSILON);
        assert!(!intv.is_empty());
    }

    #[test]
    fn test_single() {
        let intv = BoundedInterval::new_single(4);
        assert!(!intv.is_empty());
        assert!(intv.contains(&4));
        assert!(!intv.contains(&5));
    }

    #[test]
    fn test_equivalent() {
        let intv1 = BoundedInterval::lcro(1, 4);
        let intv2 = BoundedInterval::lcrc(1, 3);
        let intv3 = BoundedInterval::lcrc(1, 4);
        assert!(intv1.equivalent(&intv1));
        assert!(intv1.equivalent(&intv2));
        assert!(intv2.equivalent(&intv1));
        assert!(!intv3.equivalent(&intv1)); // same bounds, but one closed
        assert!(!intv1.equivalent(&intv3)); // same bounds, but one closed
        assert!(!intv2.equivalent(&intv3));
    }

    #[test]
    fn test_ord() {
        let b1 = Bound {
            point: 3,
            offset: OffsetSide::Left,
        };
        let b2 = Bound {
            point: 3,
            offset: OffsetSide::Right,
        };
        assert!(b1 != b2);
        assert!(b1 < b2);

        let b3 = Bound {
            point: 4,
            offset: OffsetSide::Left,
        };
        assert!(b3 == b2);
        assert!(b2 == b3);
    }

    #[test]
    fn test_left_of() {
        let intv1 = BoundedInterval::lcro(3, 5);
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

        let intv2 = BoundedInterval::lcrc(3, 5);
        assert!(intv2.left_of(&6));
        assert!(intv2.left_of(&5));
        assert!(!intv2.strictly_left_of(&5));
    }
}
