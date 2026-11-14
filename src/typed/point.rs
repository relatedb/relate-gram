//! The `point` tag: extended well-known text with an SRID (ADR-011 section 2.2).

use super::PointContent;

/// A coordinate: an optional `-`, digits, and optionally a dot and digits. No exponent.
fn coordinate(s: &str, what: &str) -> Result<f64, String> {
    let body = s.strip_prefix('-').unwrap_or(s);
    let (int, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || frac.is_some_and(|f| !digits(f)) {
        return Err(format!(
            "the {what} `{s}` is not a decimal number (digits, with an optional `-` and fraction; no exponent)"
        ));
    }
    s.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("the {what} `{s}` is not a finite number"))
}

pub(crate) fn point(text: &str) -> Result<PointContent, String> {
    let rest = text
        .strip_prefix("SRID=")
        .ok_or_else(|| format!("`{text}` has no SRID: write it as `SRID=4326;POINT(x y)`"))?;
    let (srid_text, shape) = rest
        .split_once(';')
        .ok_or_else(|| format!("`{text}` has no `;` after the SRID"))?;
    if srid_text.is_empty() || !srid_text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("the SRID `{srid_text}` is not a number"));
    }
    let srid: i64 = srid_text
        .parse()
        .map_err(|_| format!("the SRID `{srid_text}` is not a number"))?;
    let three_d_srid = match srid {
        4326 | 7203 => false,
        4979 | 9157 => true,
        _ => {
            return Err(format!(
                "the SRID {srid} is not supported; use 4326 or 7203 (two coordinates), or 4979 or 9157 (three)"
            ))
        }
    };
    let (three_d_text, inner) = if let Some(r) = shape.strip_prefix("POINT Z(") {
        (true, r)
    } else if let Some(r) = shape.strip_prefix("POINT(") {
        (false, r)
    } else {
        return Err(format!("`{shape}` is not `POINT(x y)` or `POINT Z(x y z)`"));
    };
    let inner = inner
        .strip_suffix(')')
        .ok_or_else(|| format!("`{shape}` is missing its closing parenthesis"))?;
    if three_d_text != three_d_srid {
        return Err(format!(
            "the SRID {srid} takes {} coordinates, but the point has {}",
            if three_d_srid { "three" } else { "two" },
            if three_d_text { "three" } else { "two" }
        ));
    }
    let parts: Vec<&str> = inner.split(' ').collect();
    let want = if three_d_srid { 3 } else { 2 };
    if parts.len() != want {
        return Err(format!(
            "the point has {} coordinate(s) separated by single spaces, expected {want}",
            parts.len()
        ));
    }
    let x = coordinate(parts[0], "x coordinate")?;
    let y = coordinate(parts[1], "y coordinate")?;
    let z = if three_d_srid {
        Some(coordinate(parts[2], "z coordinate")?)
    } else {
        None
    };
    if matches!(srid, 4326 | 4979) && !(-90.0..=90.0).contains(&y) {
        return Err(format!("the latitude {y} is outside [-90, 90]"));
    }
    Ok(PointContent { srid, x, y, z })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_four_srids() {
        let p = point("SRID=4326;POINT(-74.0 40.7)").unwrap();
        assert_eq!((p.srid, p.x, p.y, p.z), (4326, -74.0, 40.7, None));
        let p = point("SRID=7203;POINT(1 2)").unwrap();
        assert_eq!((p.srid, p.x, p.y, p.z), (7203, 1.0, 2.0, None));
        let p = point("SRID=4979;POINT Z(1 2 3)").unwrap();
        assert_eq!((p.srid, p.z), (4979, Some(3.0)));
        let p = point("SRID=9157;POINT Z(1.5 2.5 3.5)").unwrap();
        assert_eq!((p.srid, p.x, p.y, p.z), (9157, 1.5, 2.5, Some(3.5)));
    }

    #[test]
    fn rejects_the_rest() {
        for bad in [
            "POINT(1 2)",
            "SRID=1234;POINT(1 2)",
            "SRID=4326;POINT Z(1 2 3)",
            "SRID=4979;POINT(1 2)",
            "SRID=7203;POINT(1e3 2)",
            "SRID=7203;POINT(1 2",
            "SRID=7203;POINT(1)",
            "SRID=7203;POINT(a b)",
            "SRID=4326;POINT(0 95)",
            "SRID=4326;POINT(0 -90.5)",
            "SRID=7203;POINT(NaN 1)",
            "SRID=7203;point(1 2)",
            "",
        ] {
            assert!(point(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn latitude_bounds_are_inclusive() {
        assert!(point("SRID=4326;POINT(0 90)").is_ok());
        assert!(point("SRID=4326;POINT(0 -90)").is_ok());
    }
}
