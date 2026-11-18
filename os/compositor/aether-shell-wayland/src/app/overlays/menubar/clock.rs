//! Civil time for the menu bar clock and the Calendar app.
//!
//! The shell has no timezone database, so the clock reads the system clock in
//! UTC. That keeps the chrome dependency-free and deterministic under test;
//! `Clock::from_unix` is a pure function for the same reason.

use std::time::{SystemTime, UNIX_EPOCH};

const WEEKDAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A wall-clock instant broken into calendar fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub year: i32,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
    /// 0 = Sunday.
    pub weekday: usize,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Clock {
    pub fn now() -> Self {
        // `AETHER_FIXED_UNIX` pins the clock for screenshot comparison: the
        // reference demo and the compositor must agree on the date and time.
        if let Ok(fixed) = std::env::var("AETHER_FIXED_UNIX") {
            if let Ok(seconds) = fixed.parse::<i64>() {
                return Self::from_unix(seconds);
            }
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or(0);
        Self::from_unix(seconds)
    }

    pub fn from_unix(seconds: i64) -> Self {
        let days = seconds.div_euclid(86_400);
        let rest = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            weekday: (days + 4).rem_euclid(7) as usize,
            hour: (rest / 3_600) as u32,
            minute: ((rest % 3_600) / 60) as u32,
            second: (rest % 60) as u32,
        }
    }

    /// Midnight on the first of this clock's month, as a new clock.
    pub fn month_start(self) -> Self {
        Self { day: 1, ..self }
    }

    pub fn month_name(&self) -> &'static str {
        MONTHS[(self.month - 1) as usize]
    }

    /// `Wednesday, 18 November`.
    pub fn long_date(self) -> String {
        format!(
            "{}, {} {}",
            WEEKDAYS_LONG[self.weekday],
            self.day,
            self.month_name()
        )
    }

    /// `November 2026`.
    pub fn month_label(self) -> String {
        format!("{} {}", self.month_name(), self.year)
    }

    /// `Wed, Nov 18` — the menu bar date.
    pub fn short_date(self) -> String {
        format!(
            "{}, {} {}",
            WEEKDAYS_SHORT[self.weekday],
            &self.month_name()[..3],
            self.day
        )
    }

    /// `04:47 PM` — the menu bar time.
    pub fn time_label(self) -> String {
        let mut hour = self.hour % 12;
        if hour == 0 {
            hour = 12;
        }
        let meridiem = if self.hour < 12 { "AM" } else { "PM" };
        format!("{hour:02}:{:02} {meridiem}", self.minute)
    }

    /// Stable `YYYY-MM-DD` key used to compare days.
    pub fn day_key(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Days in the clock's month.
    pub fn days_in_month(self) -> u32 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(self.year) => 29,
            _ => 28,
        }
    }

    /// Weekday of the first day of the clock's month (0 = Sunday).
    pub fn first_weekday(self) -> usize {
        let days = days_from_civil(self.year, self.month, 1);
        (days + 4).rem_euclid(7) as usize
    }

    /// Move `months` months from this clock, clamped to day 1.
    pub fn add_months(self, months: i32) -> Self {
        let total = (self.year * 12 + self.month as i32 - 1) + months;
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12) as u32 + 1;
        Self {
            year,
            month,
            day: 1,
            weekday: 0,
            ..self
        }
        .normalized()
    }

    /// Move `days` days from this clock.
    pub fn add_days(self, days: i32) -> Self {
        let target = days_from_civil(self.year, self.month, self.day) + days as i64;
        let (year, month, day) = civil_from_days(target);
        Self {
            year,
            month,
            day,
            weekday: 0,
            ..self
        }
        .normalized()
    }

    /// Fill in `weekday` from the date fields.
    pub fn normalized(mut self) -> Self {
        let days = days_from_civil(self.year, self.month, self.day);
        self.weekday = (days + 4).rem_euclid(7) as usize;
        self
    }
}

/// Weekday column headers used by the Calendar grid.
pub const WEEKDAY_HEADERS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to y/m/d.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Days since 1970-01-01 for a civil date.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_thursday() {
        let clock = Clock::from_unix(0);
        assert_eq!((clock.year, clock.month, clock.day), (1970, 1, 1));
        assert_eq!(clock.weekday, 4);
        assert_eq!(clock.time_label(), "12:00 AM");
    }

    #[test]
    fn formats_the_demo_reference_instant() {
        // 2026-11-18 16:47 UTC.
        let clock = Clock::from_unix(1_795_020_420);
        assert_eq!(clock.year, 2026);
        assert_eq!(clock.month, 11);
        assert_eq!(clock.day, 18);
        assert_eq!(clock.short_date(), "Wed, Nov 18");
        assert_eq!(clock.time_label(), "04:47 PM");
        assert_eq!(clock.long_date(), "Wednesday, 18 November");
    }

    #[test]
    fn leap_days() {
        let leap = Clock {
            year: 2024,
            month: 2,
            day: 1,
            weekday: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
        .normalized();
        assert_eq!(leap.days_in_month(), 29);
        assert_eq!(
            Clock {
                year: 2023,
                month: 2,
                day: 1,
                weekday: 0,
                hour: 0,
                minute: 0,
                second: 0,
            }
            .normalized()
            .days_in_month(),
            28
        );
    }

    #[test]
    fn month_navigation_wraps_years() {
        let clock = Clock::from_unix(1_795_020_420).month_start();
        assert_eq!(clock.month_label(), "November 2026");
        assert_eq!(clock.add_months(1).month_label(), "December 2026");
        assert_eq!(clock.add_months(2).month_label(), "January 2027");
        assert_eq!(clock.add_months(-11).month_label(), "December 2025");
    }

    #[test]
    fn first_weekday_matches_the_calendar() {
        let november = Clock {
            year: 2026,
            month: 11,
            day: 1,
            weekday: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
        .normalized();
        // 1 November 2026 is a Sunday.
        assert_eq!(november.first_weekday(), 0);
        assert_eq!(november.day, 1);
    }

    #[test]
    fn day_arithmetic_crosses_month_boundaries() {
        let last = Clock {
            year: 2026,
            month: 11,
            day: 30,
            weekday: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
        .normalized();
        assert_eq!(last.add_days(1).day_key(), "2026-12-01");
    }
}
