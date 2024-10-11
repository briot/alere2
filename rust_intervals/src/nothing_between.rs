#[cfg(feature = "chrono")]
use chrono::{DateTime, TimeZone};

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
    fn nothing_between(&self, other: &f32) -> bool {
        // In general, comparing with EPSILON is wrong.  There are however two
        // cases:
        // * the user has used V + EPSILON with a large V.  The addition had no
        //   effect, and this sum is equal to V.

        // Note that this is incorrect for large values of floats, since adding
        // EPSILON has no effect.
        self + f32::EPSILON >= *other
    }
}
impl NothingBetween for f64 {
    fn nothing_between(&self, other: &f64) -> bool {
        self + f64::EPSILON >= *other
    }
}

impl NothingBetween for char {
    fn nothing_between(&self, other: &Self) -> bool {
        (*other as u32) - (*self as u32) <= 1
    }
}

// Blanket implementation so that we can create intervals of references (used in
// particular to avoid cloning in the implementation of left_of)
impl<T: NothingBetween> NothingBetween for &T {
    fn nothing_between(&self, other: &Self) -> bool {
        (*self).nothing_between(*other)
    }
}

#[cfg(feature = "chrono")]
impl<T: TimeZone> NothingBetween for DateTime<T> {
    fn nothing_between(&self, _other: &DateTime<T>) -> bool {
        false
    }
}

