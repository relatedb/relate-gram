//! The Relate profile of gram-value's known tags (ADR-012 section 2.3).
//!
//! gram-value's forms describe meaning in general. Relate's types need a little more: a time
//! must say its offset (Relate would otherwise fill in UTC without saying so), and a point must
//! name one of the reference systems Relate supports, with the matching number of coordinates.

use gram_value::tags::{Interpretation, Typed};

/// Check an interpreted tag against the profile. The error is a fault message for a
/// `value.tag-content` diagnostic.
pub(super) fn check(i: &Interpretation) -> Result<(), String> {
    match (&i.typed, i.tag) {
        (Typed::Time(t), "time") if t.offset.is_none() => Err(needs("time", "an offset")),
        (Typed::DateTime(d), "datetime") if d.time.offset.is_none() => {
            Err(needs("datetime", "an offset"))
        }
        (Typed::Point(p), _) => {
            let Some(srid) = p.srid else {
                return Err(needs("point", "an SRID, written `SRID=4326;x y`"));
            };
            match (srid, p.z.is_some()) {
                (4326 | 7203, false) | (4979 | 9157, true) => {}
                (4326 | 7203 | 4979 | 9157, _) => {
                    return Err(format!(
                        "SRID {srid} needs {} coordinates, not {}",
                        if matches!(srid, 4326 | 7203) {
                            "two"
                        } else {
                            "three"
                        },
                        if p.z.is_some() { "three" } else { "two" }
                    ));
                }
                _ => {
                    return Err(format!(
                        "SRID {srid} is not supported; use 4326, 7203, 4979 or 9157"
                    ));
                }
            }
            if matches!(srid, 4326 | 4979) && !(-90.0..=90.0).contains(&p.y) {
                return Err(format!("latitude {} is outside -90 to 90", p.y));
            }
            Ok(())
        }
        (Typed::Embedding(e), _) => {
            if e.dimension() > MAX_VECTOR_DIMENSION {
                return Err(format!(
                    "the dimension {} is above Relate's limit of {MAX_VECTOR_DIMENSION}; \
                     the general tag allows more, Relate does not",
                    e.dimension()
                ));
            }
            match super::elements::widen(e.dtype(), e.bytes())
                .iter()
                .position(|v| !v.is_finite())
            {
                Some(index) => Err(format!(
                    "the element at index {index} is not finite; Relate cannot store a vector \
                     with a NaN or an infinity, though the general tag carries one"
                )),
                None => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

/// The largest vector dimension Relate stores (ADR-014 section 1).
const MAX_VECTOR_DIMENSION: usize = 4096;

fn needs(tag: &str, what: &str) -> String {
    format!("a `{tag}` needs {what}; the general tag accepts this text, Relate does not")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gram_value::tags::interpret_tag;

    fn run(tag: &str, text: &str) -> Result<(), String> {
        let i = interpret_tag(tag, text)
            .expect("general form")
            .expect("known tag");
        check(&i)
    }

    #[test]
    fn time_and_datetime_need_an_offset() {
        for (tag, ok, bad) in [
            ("time", "17:35Z", "17:35"),
            ("datetime", "2024-01-15T10:30+01:00", "2024-01-15T10:30"),
        ] {
            assert_eq!(run(tag, ok), Ok(()));
            assert!(run(tag, bad).unwrap_err().contains("an offset"), "{tag}");
        }
        assert_eq!(run("time", "10:30:15.5-05:00"), Ok(()));
        assert_eq!(
            run("datetime", "2024-01-15T10:30+01:00[Europe/Paris]"),
            Ok(())
        );
    }

    #[test]
    fn point_rules() {
        for ok in [
            "SRID=4326;12.5 55.7",
            "SRID=7203;1 2",
            "SRID=4979;1 2 3",
            "SRID=9157;1 2 3",
            "SRID=7203;1 500",
        ] {
            assert_eq!(run("point", ok), Ok(()), "{ok}");
        }
        assert!(run("point", "30 10").unwrap_err().contains("an SRID"));
        assert!(run("point", "SRID=3857;1 2")
            .unwrap_err()
            .contains("not supported"));
        assert!(run("point", "SRID=4326;1 2 3")
            .unwrap_err()
            .contains("two coordinates"));
        assert!(run("point", "SRID=4979;1 2")
            .unwrap_err()
            .contains("three coordinates"));
        assert!(run("point", "SRID=4326;1 91")
            .unwrap_err()
            .contains("latitude"));
        assert!(run("point", "SRID=4979;1 -91 3")
            .unwrap_err()
            .contains("latitude"));
    }

    #[test]
    fn other_tags_have_no_profile() {
        for (tag, text) in [
            ("date", "2024-01-15"),
            ("localtime", "10:30"),
            ("localdatetime", "2024-01-15T10:30"),
            ("duration", "P1D"),
            ("uuid", "123e4567-e89b-12d3-a456-426614174000"),
        ] {
            assert_eq!(run(tag, text), Ok(()), "{tag}");
        }
    }
}
