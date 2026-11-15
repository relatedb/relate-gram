//! Typed values: the nine tags of ADR-011 section 2.2 and ADR-014 and the content forms they accept.
//!
//! Everything here works on text, with no database, so `check` can find a bad value
//! without one. The loader keeps a validated value as a `gram_value::Value::tagged` holding the normal
//! content, turns it into a Relate value with a Cypher constructor (ADR-011 section 2.7,
//! ADR-013 section 2.3), and the exporter writes one back.

pub(crate) mod elements;
mod profile;

/// The tag of a tagged string that gives it a type. Tags are matched exactly, in lower case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tag {
    Date,
    Time,
    LocalTime,
    DateTime,
    LocalDateTime,
    Duration,
    Point,
    Uuid,
    Embedding,
}

impl Tag {
    /// Every recognized tag.
    pub const ALL: [Tag; 9] = [
        Tag::Date,
        Tag::Time,
        Tag::LocalTime,
        Tag::DateTime,
        Tag::LocalDateTime,
        Tag::Duration,
        Tag::Point,
        Tag::Uuid,
        Tag::Embedding,
    ];

    /// The tag a name spells, or `None` for an unrecognized name.
    pub fn from_name(name: &str) -> Option<Tag> {
        Tag::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The spelling of the tag in gram text.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Date => "date",
            Tag::Time => "time",
            Tag::LocalTime => "localtime",
            Tag::DateTime => "datetime",
            Tag::LocalDateTime => "localdatetime",
            Tag::Duration => "duration",
            Tag::Point => "point",
            Tag::Uuid => "uuid",
            Tag::Embedding => "embedding",
        }
    }

    /// Check `text` against the content form of this tag: gram-value's interpretation, then the
    /// Relate profile (ADR-012 sections 2.2 and 2.3). The result is the normal content of the
    /// tagged string, which denotes the same value as the text; the error is a short description
    /// of the fault, for a `value.tag-content` message.
    pub fn validate(self, text: &str) -> Result<String, String> {
        let name = self.name();
        let interpretation = gram_value::tags::interpret_tag(name, text)
            .map_err(|d| {
                let prefix = format!("{name}: ");
                d.message
                    .strip_prefix(&prefix)
                    .unwrap_or(&d.message)
                    .to_string()
            })?
            .ok_or_else(|| format!("`{name}` is not a known tag"))?;
        profile::check(&interpretation)?;
        Ok(interpretation.normal_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(tag: Tag, s: &str) -> String {
        tag.validate(s).unwrap()
    }

    #[test]
    fn faults_do_not_repeat_the_tag() {
        for (tag, bad) in [
            (Tag::Date, "2024-13-45"),
            (Tag::Duration, "P1W"),
            (Tag::Uuid, "xyz"),
        ] {
            let e = tag.validate(bad).unwrap_err();
            assert!(!e.starts_with(&format!("{}: ", tag.name())), "{e}");
        }
    }

    #[test]
    fn text_tags_carry_the_normal_content() {
        assert_eq!(text(Tag::Time, "17:35Z"), "17:35:00Z");
        assert_eq!(text(Tag::Time, "10:30:00+00:00"), "10:30:00Z");
        assert_eq!(text(Tag::LocalTime, "10:30:00.50"), "10:30:00.5");
        assert_eq!(
            text(Tag::Uuid, "123E4567-E89B-12D3-A456-426614174000"),
            "123e4567-e89b-12d3-a456-426614174000"
        );
        assert!(
            text(Tag::DateTime, "2024-01-15T10:30+01:00[Europe/Paris]").contains("[Europe/Paris]")
        );
    }

    #[test]
    fn a_point_carries_its_normal_content() {
        assert_eq!(text(Tag::Point, "SRID=4979;1.5 -2 3"), "SRID=4979;1.5 -2 3");
        assert_eq!(text(Tag::Point, "SRID=4326;30.0 10"), "SRID=4326;30 10");
    }

    fn embedding(dtype: gram_value::tags::EmbeddingDtype, values: &[f64]) -> String {
        let bytes = elements::narrow(dtype, values);
        gram_value::tags::Embedding::new(dtype, values.len(), bytes)
            .unwrap()
            .to_normal_content()
    }

    #[test]
    fn embedding_is_the_ninth_tag() {
        assert_eq!(Tag::from_name("embedding"), Some(Tag::Embedding));
        assert_eq!(Tag::Embedding.name(), "embedding");
        assert_eq!(Tag::ALL.len(), 9);
    }

    #[test]
    fn an_embedding_carries_its_normal_content() {
        use gram_value::tags::EmbeddingDtype::F32;
        let normal = embedding(F32, &[1.0, -2.0]);
        assert_eq!(text(Tag::Embedding, &normal), normal);
        // Another key order and spacing gives the same normal content (gram-value corpus case).
        let nonnormal = "TwAAAAAAAAB7ICJlbWJlZGRpbmciIDogeyAic2hhcGUiIDogWzJdICwgImRhdGFfb2Zmc2V0cyIgOiBbMCwgOF0gLCAiZHR5cGUiIDogIkYzMiIgfSB9AACAPwAAAMA=";
        assert_eq!(text(Tag::Embedding, nonnormal), normal);
    }

    #[test]
    fn an_embedding_over_relates_dimension_is_an_error_naming_the_limit() {
        use gram_value::tags::EmbeddingDtype::F32;
        let at_limit = embedding(F32, &vec![0.5; 4096]);
        assert!(Tag::Embedding.validate(&at_limit).is_ok());
        let over = embedding(F32, &vec![0.5; 4097]);
        let e = Tag::Embedding.validate(&over).unwrap_err();
        assert!(e.contains("4096") && e.contains("allows more"), "{e}");
    }

    #[test]
    fn a_non_finite_element_is_an_error_naming_its_index() {
        use gram_value::tags::EmbeddingDtype::F32;
        let nan = embedding(F32, &[1.0, 2.0, f64::NAN, f64::INFINITY]);
        let e = Tag::Embedding.validate(&nan).unwrap_err();
        assert!(e.contains("index 2") && e.contains("finite"), "{e}");
    }

    #[test]
    fn the_dimension_is_reported_before_a_non_finite_element() {
        use gram_value::tags::EmbeddingDtype::F32;
        let mut values = vec![0.5; 4097];
        values[0] = f64::NAN;
        let e = Tag::Embedding
            .validate(&embedding(F32, &values))
            .unwrap_err();
        assert!(e.contains("4096") && !e.contains("finite"), "{e}");
    }

    #[test]
    fn gram_values_faults_are_passed_on() {
        let e = Tag::Embedding.validate("QQ==").unwrap_err();
        assert!(!e.starts_with("embedding: "), "{e}");
        assert!(Tag::Embedding.validate("").is_err());
    }

    #[test]
    fn the_profile_is_applied() {
        assert!(Tag::Time.validate("17:35").unwrap_err().contains("offset"));
        assert!(Tag::Point.validate("30 10").unwrap_err().contains("SRID"));
    }
}
