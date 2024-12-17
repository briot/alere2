use crate::errors::AlrError;
use anyhow::Result;
use chrono::{DateTime, Datelike, Local, MappedLocalTime, NaiveDate, TimeZone};
use rust_intervals::Interval;

/// Specifies an instant in time, that is relative to some "now".
/// Such a specification can be stored in configuration files, for instance
/// as "one year ago".  That way, when we launch the application at some point
/// in the future, this is still "one year ago".
#[derive(Clone)]
pub enum Instant {
    Epoch,
    Now,
    Armageddon,

    DaysAgo(i32),      // same time, n days ago
    StartDaysAgo(i32), // start of day, n days ago
    EndDaysAgo(i32),   // end of day, n days ago
    StartDay(String),  // start-of-day on specific date
    EndDay(String),    // end-of-day on specific date

    MonthsAgo(i32), // same time & day, n months ago (or closest day)
    StartMonthsAgo(i32), // start of month, n months ago (0 = same month)
    EndMonthsAgo(i32), // end of month, n months ago (0 = same month)

    YearsAgo(i32), // same time & day, n years ago (n can be negative)
    StartYearsAgo(i32), // start of year, n years ago (0 = current year)
    EndYearsAgo(i32), // end of year, n years ago
    StartYear(u16), // start of specific year
    EndYear(u16),  // end of specific year

    Timestamp(String), // a specific timestamp
}

impl Instant {
    /// Convert self to an actual timestamp.
    pub fn to_time(&self, now: DateTime<Local>) -> Result<DateTime<Local>> {
        let r = match self {
            Instant::Now => now,
            Instant::Epoch => DateTime::<Local>::MIN_UTC.with_timezone(&Local),
            Instant::Armageddon => {
                DateTime::<Local>::MAX_UTC.with_timezone(&Local)
            }
            Instant::DaysAgo(count) => add_days(now, -count),
            Instant::StartDaysAgo(count) => {
                start_of_day(add_days(now, -count), &Local)
            }
            Instant::EndDaysAgo(count) => {
                end_of_day(add_days(now, -count), &Local)
            }
            Instant::MonthsAgo(count) => add_months(now, -count),
            Instant::StartMonthsAgo(count) => {
                start_of_month(add_months(now, -count), &Local)?
            }
            Instant::EndMonthsAgo(count) => {
                end_of_month(add_months(now, -count), &Local)?
            }
            Instant::YearsAgo(count) => add_months(now, -count * 12),
            Instant::StartYearsAgo(count) => {
                let year = now.year() - *count;
                Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap()
            }
            Instant::EndYearsAgo(count) => {
                Local
                    .with_ymd_and_hms(now.year() - *count + 1, 1, 1, 0, 0, 0)
                    .unwrap()
                    - chrono::TimeDelta::nanoseconds(1)
            }
            Instant::StartYear(year) => {
                Local.with_ymd_and_hms(*year as i32, 1, 1, 0, 0, 0).unwrap()
            }
            Instant::EndYear(year) => {
                Local
                    .with_ymd_and_hms(*year as i32 + 1, 1, 1, 0, 0, 0)
                    .unwrap()
                    - chrono::TimeDelta::nanoseconds(1)
            }
            Instant::StartDay(date) => date
                .parse::<NaiveDate>()
                .unwrap_or_else(|_| panic!("Invalid date {}", &date))
                .and_hms_opt(00, 00, 00)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap(),
            Instant::EndDay(date) => date
                .parse::<NaiveDate>()
                .unwrap_or_else(|_| panic!("Invalid date {}", &date))
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap(),
            Instant::Timestamp(ts) => ts
                .parse::<DateTime<Local>>()
                .unwrap_or_else(|_| panic!("Invalid timestamp {}", &ts)),
        };
        Ok(r)
    }
}

impl std::fmt::Display for Instant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instant::Epoch => write!(f, "")?,
            Instant::Now => write!(f, "now")?,
            Instant::Armageddon => write!(f, "∞")?,
            Instant::DaysAgo(count) => {
                if *count == 1 {
                    write!(f, "yesterday")?
                } else {
                    write!(f, "{} days ago", count)?
                }
            }
            Instant::StartDaysAgo(count) => {
                if *count == 1 {
                    write!(f, "start of yesterday")?
                } else {
                    write!(f, "start of {} days ago", count)?
                }
            }
            Instant::EndDaysAgo(count) => {
                if *count == 1 {
                    write!(f, "end of yesterday")?
                } else {
                    write!(f, "end of {} days ago", count)?
                }
            }
            Instant::StartDay(day) => write!(f, "start of {}", day)?,
            Instant::EndDay(day) => write!(f, "end of {}", day)?,
            Instant::MonthsAgo(count) => {
                if *count == 1 {
                    write!(f, "last month")?
                } else {
                    write!(f, "{} months ago", count)?
                }
            }
            Instant::StartMonthsAgo(count) => {
                if *count == 1 {
                    write!(f, "start of last month")?
                } else {
                    write!(f, "start of {} months ago", count)?
                }
            }
            Instant::EndMonthsAgo(count) => {
                if *count == 1 {
                    write!(f, "end of last month")?
                } else {
                    write!(f, "end of {} months ago", count)?
                }
            }
            Instant::YearsAgo(count) => {
                if *count == 1 {
                    write!(f, "last year")?
                } else {
                    write!(f, "{} years ago", count)?
                }
            }
            Instant::StartYearsAgo(count) => {
                if *count == 1 {
                    write!(f, "start of last year")?
                } else {
                    write!(f, "start of {} years ago", count)?
                }
            }
            Instant::EndYearsAgo(count) => {
                if *count == 1 {
                    write!(f, "end of last year")?
                } else {
                    write!(f, "end of {} years ago", count)?
                }
            }
            Instant::StartYear(year) => write!(f, "start of {}", year)?,
            Instant::EndYear(year) => write!(f, "end of {}", year)?,
            Instant::Timestamp(ts) => write!(f, "{}", ts)?,
        }
        Ok(())
    }
}

/// A range of time [start; end[ not including the end
pub struct TimeInterval {
    pub descr: String,
    pub intv: Interval<DateTime<Local>>,
}

/// A high-level description of time ranges
pub enum Intv {
    UpTo(Instant), // from start of time to the given instant

    LastNDays(i32), // from same time, n days ago, to now

    LastNMonths(i32), // from same day and time, n months ago, to now
    MonthAgo(i32),    // a full month: 0=current month, -1=last month,...
    Monthly { begin: Instant, end: Instant },

    LastNYears(i32), // from same date and time, n years ago, to now
    SpecificYear(u16), // one specific year (e.g. 2023)
    YearAgo(i32),    // a full year: 0=current year, -1=last year,...
    Yearly { begin: Instant, end: Instant },
}

impl Intv {
    /// Compute the time range for a given interval.
    pub fn to_ranges(&self, now: DateTime<Local>) -> Result<Vec<TimeInterval>> {
        let r = match self {
            Intv::UpTo(then) => {
                let lower = Instant::Epoch.to_time(now)?;
                let upper = then.to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: format!("{}", then),
                }]
            }
            Intv::LastNDays(count) => {
                let lower = Instant::DaysAgo(*count).to_time(now)?;
                let upper = Instant::Now.to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: if *count == 1 {
                        "Over 1 day".to_string()
                    } else {
                        format!("Over {} days", *count)
                    },
                }]
            }
            Intv::LastNMonths(count) => {
                let lower = Instant::MonthsAgo(*count).to_time(now)?;
                let upper = Instant::Now.to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: if *count == 1 {
                        "Over 1 month".to_string()
                    } else {
                        format!("Over {} months", *count)
                    },
                }]
            }
            Intv::LastNYears(count) => {
                let lower = Instant::YearsAgo(*count).to_time(now)?;
                let upper = Instant::Now.to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: if *count == 1 {
                        "Over 1 year".to_string()
                    } else {
                        format!("Over {} years", *count)
                    },
                }]
            }
            Intv::YearAgo(count) => {
                let lower = Instant::StartYearsAgo(*count).to_time(now)?;
                let upper = Instant::StartYearsAgo(*count - 1).to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: format!("{}", lower.year()),
                }]
            }
            Intv::MonthAgo(count) => {
                let lower = Instant::StartMonthsAgo(*count).to_time(now)?;
                let upper = Instant::StartMonthsAgo(*count - 1).to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: format!("{}-{}", lower.year(), lower.month()),
                }]
            }
            Intv::SpecificYear(year) => {
                let lower = Instant::StartYear(*year).to_time(now)?;
                let upper = Instant::StartYear(*year + 1).to_time(now)?;
                vec![TimeInterval {
                    intv: Interval::new_closed_open(lower, upper),
                    descr: format!("{}", lower.year()),
                }]
            }
            Intv::Yearly { begin, end } => {
                let mut result = Vec::new();
                let mut year = begin.to_time(now)?.year() as u16;
                let end_year = end.to_time(now)?.year() as u16;
                while year <= end_year {
                    let lower = Instant::StartYear(year).to_time(now)?;
                    let upper = Instant::StartYear(year + 1).to_time(now)?;
                    result.push(TimeInterval {
                        intv: Interval::new_closed_open(lower, upper),
                        descr: format!("{}", year),
                    });
                    year += 1;
                }
                result
            }
            Intv::Monthly { begin, end } => {
                let mut result = Vec::new();
                let mut current = start_of_month(begin.to_time(now)?, &Local)?;
                let end = end_of_month(end.to_time(now)?, &Local)?;
                while current <= end {
                    let next_start = start_of_month(
                        current + chrono::Months::new(1),
                        &Local,
                    )?;
                    result.push(TimeInterval {
                        intv: Interval::new_closed_open(current, next_start),
                        descr: format!(
                            "{}-{}",
                            current.year(),
                            current.month()
                        ),
                    });
                    current = next_start;
                }
                result
            }
        };
        Ok(r)
    }
}

/// Returns the same day and time, a number of months in the future or past.
/// If a day doesn't exist in the target month, it returns the last valid day
/// of that month.
fn add_months<TZ: TimeZone>(d: DateTime<TZ>, count: i32) -> DateTime<TZ> {
    match count.cmp(&0) {
        std::cmp::Ordering::Equal => d,
        std::cmp::Ordering::Less => d - chrono::Months::new(-count as u32),
        std::cmp::Ordering::Greater => d + chrono::Months::new(count as u32),
    }
}

/// Return the same time, count days ago
fn add_days<TZ: TimeZone>(d: DateTime<TZ>, count: i32) -> DateTime<TZ> {
    match count.cmp(&0) {
        std::cmp::Ordering::Equal => d,
        std::cmp::Ordering::Less => d - chrono::Days::new(-count as u64),
        std::cmp::Ordering::Greater => d + chrono::Days::new(count as u64),
    }
}

/// Return the start of day
fn start_of_day<TZ: TimeZone>(d: DateTime<TZ>, tz: &TZ) -> DateTime<TZ> {
    match tz.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0) {
        MappedLocalTime::Single(t) => t,
        MappedLocalTime::Ambiguous(t1, _) => t1,
        MappedLocalTime::None => d,
    }
}

/// Return the end of day
fn end_of_day<TZ: TimeZone>(d: DateTime<TZ>, tz: &TZ) -> DateTime<TZ> {
    let s = match tz.with_ymd_and_hms(d.year(), d.month(), d.day(), 23, 59, 59)
    {
        MappedLocalTime::Single(t) => t,
        MappedLocalTime::Ambiguous(t1, _) => t1,
        MappedLocalTime::None => d,
    };
    s + chrono::Duration::nanoseconds(999_999_999)
}

/// Return the timestamp for the first second of a month.
/// It solves ambiguities (e.g. a time that would fall during daylight saving
/// change) by returning the earliest of the two dates.
fn start_of_month<TZ: TimeZone>(
    d: DateTime<TZ>,
    tz: &TZ,
) -> Result<DateTime<TZ>> {
    match tz.with_ymd_and_hms(d.year(), d.month(), 1, 0, 0, 0) {
        MappedLocalTime::Single(t) => Ok(t),
        MappedLocalTime::Ambiguous(t1, _) => Ok(t1),
        MappedLocalTime::None => {
            Err(AlrError::Str("Cannot compute start of month".into()))?
        }
    }
}

/// Return the last timestamp of the month.
fn end_of_month<TZ: TimeZone>(
    d: DateTime<TZ>,
    tz: &TZ,
) -> Result<DateTime<TZ>> {
    let sm = start_of_month(d.clone(), tz)?;
    let sd = start_of_day(sm, tz);
    let next_month = add_months(sd, 1);
    Ok(next_month - chrono::TimeDelta::nanoseconds(1))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::times::Instant;
    use chrono::{DateTime, FixedOffset, Local, TimeZone, Utc};

    fn intv_to_string(intv: Intv, now: DateTime<Local>) -> Result<Vec<String>> {
        Ok(intv
            .to_ranges(now)?
            .iter()
            .map(|intv| format!("{}", intv.intv))
            .collect::<Vec<String>>())
    }

    #[test]
    fn test_instant() -> Result<()> {
        // Output timezone uses a fixed offset so the tests succeed wherever
        // we run them.
        let tz = FixedOffset::east_opt(4 * 3600).unwrap();
        let sep_10 = "2024-09-10 12:00:00Z".parse::<DateTime<Local>>().unwrap();
        let aug_31 = "2024-08-31 12:00:00Z".parse::<DateTime<Local>>().unwrap();

        assert_eq!(
            Instant::Now.to_time(sep_10)?.with_timezone(&tz).to_string(),
            "2024-09-10 16:00:00 +04:00",
        );
        assert_eq!(
            Instant::DaysAgo(1)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2024-09-09 16:00:00 +04:00",
        );
        assert_eq!(
            Instant::DaysAgo(-5)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2024-09-15 16:00:00 +04:00",
        );
        assert_eq!(
            Instant::MonthsAgo(1)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2024-08-10 16:00:00 +04:00",
        );
        assert_eq!(
            Instant::MonthsAgo(-5)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2025-02-10 17:00:00 +04:00",
        );
        assert_eq!(
            Instant::MonthsAgo(2)
                .to_time(aug_31)?
                .with_timezone(&tz)
                .to_string(),
            "2024-06-30 16:00:00 +04:00", // closest day
        );
        assert_eq!(
            Instant::MonthsAgo(6)
                .to_time(aug_31)?
                .with_timezone(&tz)
                .to_string(),
            "2024-02-29 17:00:00 +04:00", // closest day
        );
        assert_eq!(
            Instant::StartMonthsAgo(6)
                .to_time(aug_31)?
                .with_timezone(&tz)
                .to_string(),
            "2024-02-01 03:00:00 +04:00", // closest day
        );

        // End of month: in the local timezone, it is 2024-02-29, but we output
        // in a different calendar.
        assert_eq!(
            Instant::EndMonthsAgo(6)
                .to_time(aug_31)?
                .with_timezone(&tz)
                .to_string(),
            "2024-03-01 02:59:59.999999999 +04:00", // closest day
        );
        assert_eq!(
            Instant::YearsAgo(1)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2023-09-10 16:00:00 +04:00",
        );
        assert_eq!(
            Instant::YearsAgo(-5)
                .to_time(sep_10)?
                .with_timezone(&tz)
                .to_string(),
            "2029-09-10 16:00:00 +04:00",
        );
        Ok(())
    }

    #[test]
    fn test_interval() -> Result<()> {
        let sep01 = "2024-09-01 12:00:00Z".parse::<DateTime<Local>>().unwrap();
        assert_eq!(
            intv_to_string(
                Intv::Yearly {
                    begin: Instant::StartYear(2022),
                    end: Instant::StartYear(2024),
                },
                sep01
            )?,
            vec![
                "[2022-01-01 00:00:00 +01:00, 2023-01-01 00:00:00 +01:00)"
                    .to_string(),
                "[2023-01-01 00:00:00 +01:00, 2024-01-01 00:00:00 +01:00)"
                    .to_string(),
                "[2024-01-01 00:00:00 +01:00, 2025-01-01 00:00:00 +01:00)"
                    .to_string(),
            ],
        );
        assert_eq!(
            intv_to_string(Intv::MonthAgo(2), sep01)?,
            vec!["[2024-07-01 00:00:00 +02:00, 2024-08-01 00:00:00 +02:00)"
                .to_string(),],
        );
        assert_eq!(
            intv_to_string(Intv::YearAgo(2), sep01)?,
            vec!["[2022-01-01 00:00:00 +01:00, 2023-01-01 00:00:00 +01:00)"
                .to_string(),],
        );
        assert_eq!(
            intv_to_string(Intv::SpecificYear(1999), sep01)?,
            vec!["[1999-01-01 00:00:00 +01:00, 2000-01-01 00:00:00 +01:00)"
                .to_string(),],
        );
        assert_eq!(
            intv_to_string(Intv::LastNYears(10), sep01)?,
            vec!["[2014-09-01 14:00:00 +02:00, 2024-09-01 14:00:00 +02:00)"
                .to_string(),],
        );
        assert_eq!(
            intv_to_string(
                Intv::Monthly {
                    begin: Instant::MonthsAgo(2),
                    end: Instant::MonthsAgo(-1),
                },
                sep01
            )?,
            vec![
                "[2024-07-01 00:00:00 +02:00, 2024-08-01 00:00:00 +02:00)"
                    .to_string(),
                "[2024-08-01 00:00:00 +02:00, 2024-09-01 00:00:00 +02:00)"
                    .to_string(),
                "[2024-09-01 00:00:00 +02:00, 2024-10-01 00:00:00 +02:00)"
                    .to_string(),
                "[2024-10-01 00:00:00 +02:00, 2024-11-01 00:00:00 +01:00)"
                    .to_string(),
            ],
        );
        Ok(())
    }

    #[test]
    fn test_add_days() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 31, 12, 0, 0).unwrap();
        assert_eq!(add_days(dt, 0), dt,);
        assert_eq!(
            add_days(dt, 1),
            Utc.with_ymd_and_hms(2024, 2, 1, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_days(dt, 2),
            Utc.with_ymd_and_hms(2024, 2, 2, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_days(dt, -1),
            Utc.with_ymd_and_hms(2024, 1, 30, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_days(dt, -2),
            Utc.with_ymd_and_hms(2024, 1, 29, 12, 00, 00).unwrap(),
        );
    }

    #[test]
    fn test_add_months() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 31, 12, 0, 0).unwrap();
        assert_eq!(add_months(dt, 0), dt,);
        assert_eq!(
            add_months(dt, 1),
            Utc.with_ymd_and_hms(2024, 2, 29, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_months(dt, 2),
            Utc.with_ymd_and_hms(2024, 3, 31, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_months(dt, 3),
            Utc.with_ymd_and_hms(2024, 4, 30, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_months(dt, -1),
            Utc.with_ymd_and_hms(2023, 12, 31, 12, 00, 00).unwrap(),
        );
        assert_eq!(
            add_months(dt, -2),
            Utc.with_ymd_and_hms(2023, 11, 30, 12, 00, 00).unwrap(),
        );
    }

    #[test]
    fn test_end_of_day() {
        let dt = Utc.with_ymd_and_hms(2024, 9, 18, 12, 0, 0).unwrap();
        let eod = end_of_day(dt, &Utc);
        assert_eq!(
            eod,
            Utc.with_ymd_and_hms(2024, 9, 18, 23, 59, 59).unwrap()
                + chrono::Duration::nanoseconds(999_999_999)
        );

        let dt = Local.with_ymd_and_hms(2024, 9, 18, 12, 0, 0).unwrap();
        let eod = end_of_day(dt, &Local);
        assert_eq!(
            eod,
            Local.with_ymd_and_hms(2024, 9, 18, 23, 59, 59).unwrap()
                + chrono::Duration::nanoseconds(999_999_999)
        );

        // Leap second are not supported by chrono, not relevant for us.
        let dt = Local.with_ymd_and_hms(2016, 12, 31, 12, 0, 0).unwrap();
        let eod = end_of_day(dt, &Local);
        assert_eq!(
            eod,
            Local.with_ymd_and_hms(2016, 12, 31, 23, 59, 59).unwrap()
                + chrono::Duration::nanoseconds(999_999_999)
        );
    }
}
