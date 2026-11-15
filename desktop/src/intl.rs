//! Locale formatting equivalent to the subset of `Intl` the interface used.
//! `intl.json` is generated from ICU by `scripts/generate-intl.mjs`.
use chrono::{Datelike, Local, TimeZone, Timelike};
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
#[serde(untagged)]
enum Part {
    Field(String),
    Literal { literal: String },
}

#[derive(Deserialize)]
struct Units {
    millisecond: String,
    second: String,
    minute: String,
}

#[derive(Deserialize)]
struct Periods {
    am: Option<String>,
    pm: Option<String>,
}

#[derive(Deserialize)]
struct Conventions {
    decimal: String,
    group: String,
    grouping: String,
    min_grouping: u8,
    calendar: String,
    digits: String,
    date_long: Vec<Part>,
    date_short: Vec<Part>,
    months_long: Vec<String>,
    months_short: Vec<String>,
    periods: Periods,
    units: Units,
}

fn table() -> &'static HashMap<String, Conventions> {
    static TABLE: OnceLock<HashMap<String, Conventions>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(include_str!("intl.json")).expect("intl.json"))
}

fn conventions(locale: &str) -> &'static Conventions {
    let table = table();
    table.get(locale).unwrap_or_else(|| &table["en"])
}

fn localize_digits(text: &str, digits: &str) -> String {
    if digits == "0123456789" {
        return text.to_owned();
    }
    let native: Vec<char> = digits.chars().collect();
    text.chars()
        .map(|c| c.to_digit(10).map_or(c, |d| native[d as usize]))
        .collect()
}

fn group(integer: &str, c: &Conventions) -> String {
    let len = integer.len();
    if len < 4 || (c.min_grouping == 2 && len < 5) {
        return integer.to_owned();
    }
    let mut chunks = Vec::new();
    let mut end = len;
    let mut size = 3;
    while end > size {
        chunks.push(&integer[end - size..end]);
        end -= size;
        if c.grouping == "indian" {
            size = 2;
        }
    }
    chunks.push(&integer[..end]);
    chunks.reverse();
    chunks.join(&c.group)
}

/// `Number.prototype.toLocaleString(locale, { maximumFractionDigits })`.
pub fn number(locale: &str, value: f64, max_fraction: usize) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".into()
        } else {
            "∞".into()
        };
    }
    let c = conventions(locale);
    let factor = 10f64.powi(max_fraction as i32);
    // ICU rounds half away from zero on the decimal representation.
    let rounded = (value.abs() * factor + 1e-9).round() / factor;
    let text = format!("{rounded:.max_fraction$}");
    let (integer, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let fraction = fraction.trim_end_matches('0');
    let mut out = String::new();
    if value < 0.0 && rounded != 0.0 {
        out.push('-');
    }
    out.push_str(&group(integer, c));
    if !fraction.is_empty() {
        out.push_str(&c.decimal);
        out.push_str(fraction);
    }
    localize_digits(&out, &c.digits)
}

/// Default `toLocaleString()` for numbers (up to three fraction digits).
pub fn plain(locale: &str, value: f64) -> String {
    number(locale, value, 3)
}

pub fn integer(locale: &str, value: i64) -> String {
    number(locale, value as f64, 0)
}

fn persian(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    // Gregorian to Solar Hijri, after the algorithm by K. Borkowski.
    let g_days = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let gy2 = if month > 2 { year + 1 } else { year };
    let mut days = 355666 + 365 * year + (gy2 + 3) / 4 - (gy2 + 99) / 100
        + (gy2 + 399) / 400
        + day as i32
        + g_days[month as usize - 1];
    let mut jy = -1595 + 33 * (days / 12053);
    days %= 12053;
    jy += 4 * (days / 1461);
    days %= 1461;
    if days > 365 {
        jy += (days - 1) / 365;
        days = (days - 1) % 365;
    }
    let (jm, jd) = if days < 186 {
        (1 + days / 31, 1 + days % 31)
    } else {
        (7 + (days - 186) / 30, 1 + (days - 186) % 30)
    };
    (jy, jm as u32, jd as u32)
}

/// `Intl.DateTimeFormat` with the interface's long or compact options.
pub fn date(locale: &str, millis: i64, compact: bool) -> String {
    let c = conventions(locale);
    let Some(time) = Local.timestamp_millis_opt(millis).single() else {
        return String::new();
    };
    let (mut year, mut month, mut day) = (time.year(), time.month(), time.day());
    match c.calendar.as_str() {
        "buddhist" => year += 543,
        "persian" => (year, month, day) = persian(year, month, day),
        _ => {}
    }
    let hour = time.hour();
    let twelve = c.periods.am.is_some();
    let shown_hour = if twelve {
        match hour % 12 {
            0 => 12,
            h => h,
        }
    } else {
        hour
    };
    let parts = if compact { &c.date_short } else { &c.date_long };
    let months = if compact {
        &c.months_short
    } else {
        &c.months_long
    };
    let mut out = String::new();
    for part in parts {
        match part {
            Part::Literal { literal } => out.push_str(literal),
            Part::Field(field) => match field.as_str() {
                "year" => out.push_str(&localize_digits(&year.to_string(), &c.digits)),
                "month" => out.push_str(&months[month as usize - 1]),
                "day" if compact => out.push_str(&localize_digits(&format!("{day:02}"), &c.digits)),
                "day" => out.push_str(&localize_digits(&day.to_string(), &c.digits)),
                "hour" => out.push_str(&localize_digits(&format!("{shown_hour:02}"), &c.digits)),
                "minute" => out.push_str(&localize_digits(
                    &format!("{:02}", time.minute()),
                    &c.digits,
                )),
                "dayPeriod" => {
                    let period = if hour < 12 {
                        &c.periods.am
                    } else {
                        &c.periods.pm
                    };
                    out.push_str(period.as_deref().unwrap_or_default());
                }
                _ => {}
            },
        }
    }
    out
}

fn unit(locale: &str, value: f64, pattern: &str) -> String {
    pattern.replace("{0}", &number(locale, value, 1))
}

/// Durations as the interface displayed them (`ms`, `s`, or `min s`).
pub fn duration(locale: &str, ms: f64) -> String {
    let u = &conventions(locale).units;
    if ms < 1000.0 {
        unit(locale, ms.round(), &u.millisecond)
    } else if ms < 60000.0 {
        unit(locale, ms / 1000.0, &u.second)
    } else {
        format!(
            "{} {}",
            unit(locale, (ms / 60000.0).floor(), &u.minute),
            unit(locale, ((ms % 60000.0) / 1000.0).round(), &u.second)
        )
    }
}

/// Binary byte quantities; French uses octets (o, Kio…).
pub fn bytes(locale: &str, value: Option<f64>, digits: usize) -> String {
    let Some(value) = value.filter(|v| v.is_finite()) else {
        return crate::i18n::t("Non communiqué");
    };
    let units: [&str; 5] = if locale == "fr" {
        ["o", "Kio", "Mio", "Gio", "Tio"]
    } else {
        ["B", "KiB", "MiB", "GiB", "TiB"]
    };
    if value == 0.0 {
        return format!("{} {}", number(locale, 0.0, 0), units[0]);
    }
    let index = ((value.ln() / 1024f64.ln()).floor().max(0.0) as usize).min(units.len() - 1);
    format!(
        "{} {}",
        number(locale, value / 1024f64.powi(index as i32), digits),
        units[index]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_follow_icu_conventions() {
        assert_eq!(number("fr", 1234567.5, 3), "1\u{202f}234\u{202f}567,5");
        assert_eq!(number("en", 1234567.5, 3), "1,234,567.5");
        assert_eq!(number("es", 1234.0, 0), "1234");
        assert_eq!(number("es", 12345.0, 0), "12.345");
        assert_eq!(number("hi", 1234567.5, 3), "12,34,567.5");
        assert_eq!(number("bn", 1234.0, 0), "১,২৩৪");
        assert_eq!(number("fa", 1234567.5, 3), "۱٬۲۳۴٬۵۶۷٫۵");
        assert_eq!(number("fr", 30.649, 1), "30,6");
        assert_eq!(number("fr", 0.05, 1), "0,1");
    }

    #[test]
    fn persian_calendar_matches_icu() {
        assert_eq!(persian(2031, 11, 23), (1410, 9, 2));
        assert_eq!(persian(2024, 3, 20), (1403, 1, 1));
    }

    #[test]
    fn durations_use_short_units() {
        assert_eq!(duration("fr", 493.0), "493 ms");
        assert_eq!(duration("fr", 2500.0), "2,5 s");
        assert_eq!(duration("en", 61000.0), "1 min 1 sec");
    }

    #[test]
    fn bytes_use_locale_units() {
        assert_eq!(bytes("fr", Some(32_870_000_000.0), 1), "30,6 Gio");
        assert_eq!(bytes("en", Some(1536.0), 1), "1.5 KiB");
    }
}
