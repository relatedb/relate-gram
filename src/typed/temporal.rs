//! The temporal tags: `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`
//! (ADR-011 section 2.2).

use super::TypedContent;

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Check a `YYYY-MM-DD` date: a year of four or more digits with an optional `-`, a month
/// from 1 to 12 and a day that exists in that month.
fn check_date(text: &str) -> Result<(), String> {
    let body = text.strip_prefix('-').unwrap_or(text);
    let parts: Vec<&str> = body.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("the date `{text}` is not of the form YYYY-MM-DD"));
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    if y.len() < 4
        || !all_digits(y)
        || m.len() != 2
        || !all_digits(m)
        || d.len() != 2
        || !all_digits(d)
    {
        return Err(format!(
            "the date `{text}` is not of the form YYYY-MM-DD (a year of at least four digits, two-digit month and day)"
        ));
    }
    let year: u64 = y
        .parse()
        .map_err(|_| format!("the year `{y}` of `{text}` is too large"))?;
    let month: u32 = m.parse().unwrap_or(0);
    let day: u32 = d.parse().unwrap_or(0);
    if !(1..=12).contains(&month) {
        return Err(format!(
            "the month {month} of `{text}` is not between 1 and 12"
        ));
    }
    let days = match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day < 1 || day > days {
        return Err(format!("the month {month} of `{text}` has no day {day}"));
    }
    Ok(())
}

/// Check `hh:mm:ss` with an optional fraction of 1 to 9 digits at the start of `s`;
/// returns what follows.
fn check_clock(s: &str) -> Result<&str, String> {
    let b = s.as_bytes();
    let two = |i: usize| -> Option<u32> {
        let (a, c) = (*b.get(i)?, *b.get(i + 1)?);
        (a.is_ascii_digit() && c.is_ascii_digit())
            .then(|| u32::from(a - b'0') * 10 + u32::from(c - b'0'))
    };
    let shape_ok = b.len() >= 8 && b[2] == b':' && b[5] == b':';
    let (Some(h), Some(m), Some(sec)) = (two(0), two(3), two(6)) else {
        return Err(format!("`{s}` does not start with a time hh:mm:ss"));
    };
    if !shape_ok {
        return Err(format!("`{s}` does not start with a time hh:mm:ss"));
    }
    if h > 23 {
        return Err(format!("the hour {h} is not between 0 and 23"));
    }
    if m > 59 {
        return Err(format!("the minute {m} is not between 0 and 59"));
    }
    if sec > 59 {
        return Err(format!("the second {sec} is not between 0 and 59"));
    }
    let rest = &s[8..];
    let Some(frac) = rest.strip_prefix('.') else {
        return Ok(rest);
    };
    let n = frac.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        return Err("a `.` must be followed by 1 to 9 digits of a second".to_string());
    }
    if n > 9 {
        return Err(format!(
            "a fraction of a second has at most 9 digits, found {n}"
        ));
    }
    Ok(&frac[n..])
}

/// Check an offset `Z`, `+hh:mm` or `-hh:mm` at the start of `s`; returns what follows.
fn check_offset(s: &str) -> Result<&str, String> {
    if let Some(rest) = s.strip_prefix('Z') {
        return Ok(rest);
    }
    let b = s.as_bytes();
    if b.is_empty() {
        return Err("the offset is missing: write `Z` or `+hh:mm` or `-hh:mm`".to_string());
    }
    if !(b[0] == b'+' || b[0] == b'-') {
        return Err(format!(
            "`{s}` does not start with an offset: write `Z` or `+hh:mm` or `-hh:mm`"
        ));
    }
    let two = |i: usize| -> Option<u32> {
        let (a, c) = (*b.get(i)?, *b.get(i + 1)?);
        (a.is_ascii_digit() && c.is_ascii_digit())
            .then(|| u32::from(a - b'0') * 10 + u32::from(c - b'0'))
    };
    let (Some(h), Some(m)) = (two(1), two(4)) else {
        return Err(format!("`{s}` is not an offset of the form +hh:mm"));
    };
    if b.get(3) != Some(&b':') {
        return Err(format!("`{s}` is not an offset of the form +hh:mm"));
    }
    if h > 23 || m > 59 {
        return Err(format!("the offset `{}` is out of range", &s[..6]));
    }
    Ok(&s[6..])
}

/// Check a zone name in brackets, `[Europe/Paris]`, to the end of `s`: non-empty, and only
/// letters, digits and `/ _ + -`. Whether the name is a known zone is the database's check.
fn check_zone(s: &str) -> Result<(), String> {
    let inner = s
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or_else(|| format!("`{s}` after the offset is not a zone name in square brackets"))?;
    if inner.is_empty() {
        return Err("the zone name in square brackets is empty".to_string());
    }
    if let Some(c) = inner
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-')))
    {
        return Err(format!(
            "the zone name `{inner}` has the character `{c}`; use letters, digits and `/ _ + -`"
        ));
    }
    Ok(())
}

fn text(s: &str) -> TypedContent {
    TypedContent::Text(s.to_string())
}

pub(crate) fn date(s: &str) -> Result<TypedContent, String> {
    check_date(s)?;
    Ok(text(s))
}

pub(crate) fn time(s: &str) -> Result<TypedContent, String> {
    let rest = check_clock(s)?;
    let rest = check_offset(rest)?;
    if !rest.is_empty() {
        return Err(format!("unexpected `{rest}` after the offset"));
    }
    Ok(text(s))
}

pub(crate) fn local_time(s: &str) -> Result<TypedContent, String> {
    let rest = check_clock(s)?;
    if !rest.is_empty() {
        return Err(format!(
            "unexpected `{rest}` after the time; a local time has no offset (use the `time` tag for one)"
        ));
    }
    Ok(text(s))
}

pub(crate) fn date_time(s: &str) -> Result<TypedContent, String> {
    let (date, time) = s
        .split_once('T')
        .ok_or_else(|| format!("the datetime `{s}` has no `T` between the date and the time"))?;
    check_date(date)?;
    let rest = check_clock(time)?;
    let rest = check_offset(rest)?;
    if !rest.is_empty() {
        check_zone(rest)?;
    }
    Ok(text(s))
}

pub(crate) fn local_date_time(s: &str) -> Result<TypedContent, String> {
    let (date, time) = s.split_once('T').ok_or_else(|| {
        format!("the local datetime `{s}` has no `T` between the date and the time")
    })?;
    check_date(date)?;
    let rest = check_clock(time)?;
    if !rest.is_empty() {
        return Err(format!(
            "unexpected `{rest}` after the time; a local datetime has no offset or zone (use the `datetime` tag for one)"
        ));
    }
    Ok(text(s))
}

/// One duration component: an optional `-`, digits, and for seconds a fraction of up to 9
/// digits, then the designator letter. Returns the designator, what follows, and whether a
/// fraction was written.
fn component(s: &str, allow_fraction: bool) -> Result<(char, &str, bool), String> {
    let body = s.strip_prefix('-').unwrap_or(s);
    let n = body.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        return Err(format!(
            "`{s}` is not a duration component: expected a number then a letter"
        ));
    }
    let mut rest = &body[n..];
    let mut had_fraction = false;
    if let Some(frac) = rest.strip_prefix('.') {
        let m = frac.bytes().take_while(u8::is_ascii_digit).count();
        if !allow_fraction {
            return Err("only the seconds of a duration may have a fraction".to_string());
        }
        if m == 0 || m > 9 {
            return Err("a fraction of a second has 1 to 9 digits".to_string());
        }
        rest = &frac[m..];
        had_fraction = true;
    }
    let mut chars = rest.chars();
    let Some(d) = chars.next() else {
        return Err(
            "a duration component has a number but no letter (Y, M, D, H, M or S)".to_string(),
        );
    };
    Ok((d, chars.as_str(), had_fraction))
}

pub(crate) fn duration(s: &str) -> Result<TypedContent, String> {
    let Some(body) = s.strip_prefix('P') else {
        return Err(format!(
            "the duration `{s}` must start with an upper-case `P` (a leading `-` before the `P`, and weeks, are not accepted)"
        ));
    };
    if body.is_empty() {
        return Err("a duration needs at least one component after `P`".to_string());
    }
    let (date_part, time_part) = match body.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (body, None),
    };
    let mut count = 0;
    let mut rest = date_part;
    let mut last = 0usize;
    while !rest.is_empty() {
        let (d, r, _) = component(rest, false)?;
        let order = match d {
            'Y' => 1,
            'M' => 2,
            'D' => 3,
            'W' => return Err("weeks are not accepted in a duration; write days".to_string()),
            other => {
                return Err(format!(
                "`{other}` is not a date designator of a duration (Y, M or D); times follow a `T`"
            ))
            }
        };
        if order <= last {
            return Err(
                "the components of a duration are in the order Y, M, D, each at most once"
                    .to_string(),
            );
        }
        last = order;
        rest = r;
        count += 1;
    }
    if let Some(t) = time_part {
        if t.is_empty() {
            return Err("a `T` in a duration must be followed by H, M or S components".to_string());
        }
        let mut rest = t;
        let mut last = 0usize;
        while !rest.is_empty() {
            let (d, r, had_fraction) = component(rest, true)?;
            let order = match d {
                'H' => 1,
                'M' => 2,
                'S' => 3,
                other => {
                    return Err(format!(
                        "`{other}` is not a time designator of a duration (H, M or S)"
                    ))
                }
            };
            if had_fraction && d != 'S' {
                return Err("only the seconds of a duration may have a fraction".to_string());
            }
            if order <= last {
                return Err(
                    "the time components of a duration are in the order H, M, S, each at most once"
                        .to_string(),
                );
            }
            last = order;
            rest = r;
            count += 1;
        }
    }
    if count == 0 {
        return Err("a duration needs at least one component".to_string());
    }
    Ok(text(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(f: fn(&str) -> Result<TypedContent, String>, accepted: &[&str]) {
        for s in accepted {
            assert!(f(s).is_ok(), "should accept {s}: {:?}", f(s));
        }
    }
    fn bad(f: fn(&str) -> Result<TypedContent, String>, rejected: &[&str]) {
        for s in rejected {
            assert!(f(s).is_err(), "should reject {s:?}");
        }
    }

    #[test]
    fn date_forms() {
        ok(
            date,
            &[
                "2024-01-15",
                "-0044-03-15",
                "12345-06-07",
                "2024-02-29",
                "0000-01-01",
            ],
        );
        bad(
            date,
            &[
                "2024-13-15",
                "2024-00-15",
                "2024-01-32",
                "2023-02-29",
                "2024-04-31",
                "2024-W03-1",
                "2024-015",
                "20240115",
                "+2024-01-15",
                "24-01-15",
                "2024-1-15",
                "2024-01-15T10:00:00",
                "",
                " 2024-01-15",
            ],
        );
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        ok(date, &["2000-02-29", "2024-02-29", "-0004-02-29"]);
        bad(date, &["1900-02-29", "2023-02-29", "2100-02-29"]);
    }

    #[test]
    fn time_forms() {
        ok(
            time,
            &[
                "10:30:00Z",
                "10:30:00+01:00",
                "10:30:00.5-05:30",
                "10:30:00.123456789+05:30",
                "23:59:59Z",
            ],
        );
        bad(
            time,
            &[
                "10:30:00",
                "10:30Z",
                "10:30:60Z",
                "24:00:00Z",
                "10:60:00Z",
                "10:30:00.1234567890Z",
                "10:30:00.Z",
                "10:30:00+0100",
                "10:30:00+1:00",
                "10:30:00z",
                "",
                "10:30:00+24:00",
            ],
        );
    }

    #[test]
    fn local_time_forms() {
        ok(
            local_time,
            &["10:30:00", "00:00:00.000000001", "23:59:59.9"],
        );
        bad(
            local_time,
            &[
                "10:30",
                "10:30:00Z",
                "10:30:00+01:00",
                "25:00:00",
                "10:30:00.",
                "10:30:60",
            ],
        );
    }

    #[test]
    fn date_time_forms() {
        ok(
            date_time,
            &[
                "2024-01-15T10:30:00Z",
                "2024-01-15T10:30:00+01:00",
                "2024-01-15T10:30:00.123456789Z",
                "2024-01-15T10:30:00+01:00[Europe/Paris]",
                "2024-01-15T10:30:00Z[UTC]",
                "2024-01-15T10:30:00-05:00[America/Argentina/Buenos_Aires]",
                "2024-01-15T10:30:00+05:00[Etc/GMT+5]",
            ],
        );
        bad(
            date_time,
            &[
                "2024-01-15",
                "2024-01-15T10:30:00",
                "2024-01-15T10:30Z",
                "2024-01-15 10:30:00Z",
                "2024-01-15T10:30:60Z",
                "2024-13-15T10:30:00Z",
                "2024-01-15T10:30:00Z[]",
                "2024-01-15T10:30:00Z[Europe/Pa ris]",
                "2024-01-15T10:30:00Z[Europe/Paris",
                "2024-01-15T10:30:00Z[é]",
                "2024-01-15T10:30:00Z[Europe/Paris]x",
                "",
            ],
        );
    }

    #[test]
    fn local_date_time_forms() {
        ok(
            local_date_time,
            &["2024-01-15T10:30:00", "2024-01-15T10:30:00.123456789"],
        );
        bad(
            local_date_time,
            &[
                "2024-01-15",
                "2024-01-15T10:30",
                "2024-01-15T10:30:00Z",
                "2024-01-15T10:30:00+01:00",
                "2024-01-15T10:30:00[UTC]",
                "2023-02-29T10:30:00",
            ],
        );
    }

    #[test]
    fn duration_forms() {
        ok(
            duration,
            &[
                "P1DT2H",
                "P1Y2M3DT4H5M6.789S",
                "P-1DT-2H",
                "P1DT-2H",
                "PT0.123456789S",
                "P14D",
                "PT1S",
                "P1Y",
                "P0D",
                "PT-0.5S",
                "P1M",
                "PT1M",
                "P1Y2M",
                "PT1H30M",
            ],
        );
        bad(
            duration,
            &[
                "P",
                "PT",
                "P2W",
                "P1W2D",
                "-P1DT2H",
                "p1d",
                "P1.5D",
                "P1DT2.5H",
                "P1Y2D3M",
                "PT1S2M",
                "P1DT",
                "1DT2H",
                "PT0.1234567890S",
                "PT.5S",
                "PT5.S",
                "P1D2H",
                "",
                "P 1D",
                "P+1D",
            ],
        );
    }
}
