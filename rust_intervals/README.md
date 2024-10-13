Rust intervals
==============

This package provides various functions to deal with intervals (aka ranges)
of values.

It provides the following features:

  - Generic over the interval type.  Supports out of the box for u8, u16,
    u32, u64, i8, i16, i32, i64, char, and optionally for chrono::DateTime
  - Extensive testing, with coverage of nearly 100% of the code
  - Supports any variation on open-closed, open-open, closed-open,
    closed-closed, unbounded-open, unbounded-closed, open-unbounded, and
    close-unbounded intervals.  These intervals can be freely combined.
