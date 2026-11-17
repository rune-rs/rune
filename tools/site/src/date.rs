use std::fmt;

use anyhow::{bail, Result};

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

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    pub fn from_toml(value: &toml::value::Datetime) -> Result<Self> {
        let Some(date) = value.date else {
            bail!("`{value}` has no date");
        };

        Ok(Self {
            year: date.year.into(),
            month: date.month,
            day: date.day,
        })
    }

    /// `October 10, 2023`.
    pub fn long(&self) -> String {
        let month = MONTHS[usize::from(self.month - 1)];
        format!("{month} {}, {}", self.day, self.year)
    }

    /// RFC 2822, as RSS wants it.
    pub fn rfc2822(&self) -> String {
        let month = &MONTHS[usize::from(self.month - 1)][..3];
        let weekday = DAYS[self.days_since_epoch().rem_euclid(7).wrapping_add(3) as usize % 7];
        format!(
            "{weekday}, {:02} {month} {} 00:00:00 +0000",
            self.day, self.year
        )
    }

    /// Days since 1970-01-01, which was a Thursday.
    fn days_since_epoch(&self) -> i64 {
        let (y, m, d) = (
            i64::from(self.year),
            i64::from(self.month),
            i64::from(self.day),
        );
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }
}

/// `2023-10-10`.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
