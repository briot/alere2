use chrono::{DateTime, Local};

/// Specifies an instant in time, that is relative to some "now".
/// Such a specification can be stored in configuration files, for instance
/// as "one year ago".  That way, when we launch the application at some point
/// in the future, this is still "one year ago".
pub enum Instant {
    Now,
    DaysAgo(u64),
    MonthsAgo(u32),
    YearsAgo(u32),
}

impl Instant {

    /// Convert self to an actual timestamp.
    pub fn to_time(&self, now: DateTime<Local>) -> DateTime<Local> {
        match self {
            Instant::Now => now,
            Instant::DaysAgo(count) => now - chrono::Days::new(*count),
            Instant::MonthsAgo(count) => now - chrono::Months::new(*count),
            Instant::YearsAgo(count) => now - chrono::Months::new(*count * 12),
        }
    }
}

/// A range of time [start; end] including both ends
pub struct Range {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

impl Range {
    pub fn new(start: DateTime<Local>, end: DateTime<Local>) -> Self {
        Range { start, end }
    }
}

pub enum Interval {
    Days(u64),
    Months(u32),
    Years(u32),
}

impl Interval {

    /// Compute the time range for a given interval.  The output doesn't
    /// depend on a specific "now", so it can be reused
    pub fn to_range(&self, now: DateTime<Local>) -> Range {
        match self {
            Interval::Days(count) => {
                Range::new(
                    Instant::DaysAgo(*count).to_time(now),
                    Instant::Now.to_time(now))
            }
            Interval::Months(count) => {
                Range::new(
                    Instant::MonthsAgo(*count).to_time(now),
                    Instant::Now.to_time(now))
            }
            Interval::Years(count) => {
                Range::new(
                    Instant::YearsAgo(*count).to_time(now),
                    Instant::Now.to_time(now))
            }
        }
    }
}
