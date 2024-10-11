use crate::intervals::Interval;
use crate::nothing_between::NothingBetween;

/// The result of the difference between two intervals.  This might be a
/// single interval, or two (in which case the first one is always the
/// left-most).
#[derive(PartialEq)]
pub enum MultiInterval<T: PartialOrd + NothingBetween> {
    One(Interval<T>),
    Two(Interval<T>, Interval<T>),
}
impl<T: ::core::fmt::Debug + PartialOrd + NothingBetween> ::core::fmt::Debug
    for MultiInterval<T>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            MultiInterval::One(i1) => write!(f, "{:?}", *i1)?,
            MultiInterval::Two(i1, i2) => write!(f, "({:?} + {:?})", i1, i2)?,
        };
        Ok(())
    }
}

impl<T: PartialOrd + NothingBetween> MultiInterval<T> {
    pub(crate) fn new_from_two(intv1: Interval<T>, intv2: Interval<T>) -> Self {
        if intv1.is_empty() {
            MultiInterval::One(intv2)
        } else if intv2.is_empty() {
            MultiInterval::One(intv1)
        } else {
            MultiInterval::Two(intv1, intv2)
        }
    }
}
