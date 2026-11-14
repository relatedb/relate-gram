//! The `uuid` tag: 32 hexadecimal digits in 8-4-4-4-12 groups, either case.

use super::TypedContent;

pub(crate) fn uuid(text: &str) -> Result<TypedContent, String> {
    let b = text.as_bytes();
    if b.len() != 36 {
        return Err(format!(
            "`{text}` is not a UUID: expected 36 characters (32 hexadecimal digits in groups of 8-4-4-4-12), found {}",
            b.len()
        ));
    }
    for (i, &c) in b.iter().enumerate() {
        let hyphen = matches!(i, 8 | 13 | 18 | 23);
        if hyphen && c != b'-' {
            return Err(format!(
                "`{text}` is not a UUID: expected `-` at position {}",
                i + 1
            ));
        }
        if !hyphen && !c.is_ascii_hexdigit() {
            return Err(format!(
                "`{text}` is not a UUID: `{}` at position {} is not a hexadecimal digit",
                text.get(i..).and_then(|t| t.chars().next()).unwrap_or('?'),
                i + 1
            ));
        }
    }
    Ok(TypedContent::Text(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_either_case() {
        assert!(uuid("123e4567-e89b-12d3-a456-426614174000").is_ok());
        assert!(uuid("123E4567-E89B-12D3-A456-426614174000").is_ok());
    }

    #[test]
    fn rejects_other_forms() {
        for bad in [
            "{123e4567-e89b-12d3-a456-426614174000}",
            "123e4567e89b12d3a456426614174000",
            "123e4567-e89b-12d3-a456-42661417400",
            "123e4567-e89b-12d3-a456-4266141740000",
            "123e4567-e89b-12d3-a456-42661417400g",
            "",
        ] {
            assert!(uuid(bad).is_err(), "{bad}");
        }
    }
}
