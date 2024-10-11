//! This create provides operations for mathematical intervals.
//! Such intervals include all values between two bounds.
//!
//! This library supports multiple kinds of intervals.  Let's call E the
//! set of valid values in the interval,
//!
//!  |Interval|Constructor                       |Description
//!  |--------|----------------------------------|--------------
//!  | `[A,B]`|[`Interval::new_closed_closed`]   |left-closed, right-closed
//!  | `[A,B)`|[`Interval::new_closed_open`]     |left-closed, right-open
//!  | `(A,B)`|[`Interval::new_open_open`]       |left-open, right-open
//!  | `(A,B]`|[`Interval::new_open_closed`]     |left-open, right-closed
//!  | `(,B]` |[`Interval::new_unbounded_closed`]|left-unbounded, right-closed
//!  | `(,B)` |[`Interval::new_unbounded_open`]  |left-unbounded, right-open
//!  | `[A,)` |[`Interval::new_closed_unbounded`]|left-closed, right-unbounded
//!  | `(A,)` |[`Interval::new_open_unbounded`]  |left-open, right-unbounded
//!  | `(,)`  |[`Interval::doubly_unbounded`]    |doubly unbounded
//!  | `empty`|[`Interval::default()`]           |empty
//!
//! Any type can be used for the bounds, though operations on the interval
//! depends on the traits that the bound type implements.
//!
//! Intervals on floats (like any code using float) can be tricky.  For
//! instance, the two intervals `[1.0, 100.0)` and `[1.0, 100.0 - f32:EPSILON)`
//! are not considered equivalent, since the machine thinks the two upper
//! bounds have the same value, but one of them is closed and the other is
//! open.
//!
//! Although this type is mostly intended to be used when T can be ordered,
//! it is in fact possible to define intervals using any type.  But only a few
//! functions are then available (like [`Interval::lower()`],
//! [`Interval::upper()`],...)
//!
//! Given two intervals, and assuming T is orderable, we can compute the
//! following:
//!
//! ```text
//!        [------ A ------]
//!               [----- B -------]
//!
//!        [----------------------]     Convex hull
//!        [------)                     Difference (A - B)
//!                        (------]     Difference (B - A)
//!        [------)        (------]     Symmetric difference (A ^ B)
//!               [--------]            Intersection (A & B)
//!                                     Between is empty
//!        [----------------------]     Union (A | B)
//! ```
//!
//! When the two intervals do not overlap, we can compute:
//! ```text
//!      [---A---]   [----B----]
//!
//!      [---------------------]    Convex hull
//!      [-------]                  Difference (A - B)
//!                  [---------]    Difference (B - A)
//!      [-------]   [---------]    Symmetric difference (A ^ B)
//!                                 Intersection (A & B) is empty
//!              (---)              Between
//!                                 Union (A | B) is empty, non contiguous
//! ```
//!

mod bounds;
mod intervals;
mod nothing_between;
mod multi_intervals;

pub use crate::intervals::Interval;
pub use crate::multi_intervals::MultiInterval;
pub use crate::nothing_between::NothingBetween;
