use chrono::{DateTime, Local};

pub enum Timestamp {
    Now,
    DaysAgo(u64),
    MonthsAgo(u32),
    YearsAgo(u32),
}

pub fn get_timestamps(
    when: &[Timestamp],
) -> impl Iterator<Item = DateTime<Local>> + '_ {
    let now = Local::now();
    when.iter().map(move |ts| match ts {
        Timestamp::Now => now,
        Timestamp::DaysAgo(count) => now - chrono::Days::new(*count),
        Timestamp::MonthsAgo(count) => now - chrono::Months::new(*count),
        Timestamp::YearsAgo(count) => now - chrono::Months::new(*count * 12),
    })
}

pub struct Range {
    pub start: DateTime<Local>,
    pub to: DateTime<Local>,
}

impl Range {
    pub fn new(start: Timestamp, to: Timestamp) -> Self {
        let rg = [start, to];
        let mut iter = get_timestamps(&rg);
        let s = iter.nth(1).unwrap();
        let t = iter.nth(1).unwrap();
        Range { start: s, to: t }
    }
}

pub enum Interval {
    Days(u64),
    Months(u32),
    Years(u32),
}

pub fn get_ranges(ranges: &[Interval]) -> impl Iterator<Item = Range> + '_ {
    ranges.iter().map(move |itv| match itv {
        Interval::Days(count) => {
            Range::new(Timestamp::DaysAgo(*count), Timestamp::Now)
        }
        Interval::Months(count) => {
            Range::new(Timestamp::MonthsAgo(*count), Timestamp::Now)
        }
        Interval::Years(count) => {
            Range::new(Timestamp::YearsAgo(*count), Timestamp::Now)
        }
    })
}
