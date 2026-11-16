//! The value table of ADR-015 section 2.3: both sides of a graph comparison reduce a property
//! value to a `gram_value::Value`, and the canonical text of that value is what is compared.
//!
//! The expected side maps the pattern-rs reader's values. The database side maps the pair
//! (`valueType()`, `toString()`) of a stored property, with lists and points read element by
//! element (spec 006 research R3). Neither side uses the exporter.
use gram_value::tags::{interpret_tag, Embedding, EmbeddingDtype};
use gram_value::Value;
use pattern_core::Value as PV;

fn strip_not_null(t: &str) -> &str {
    t.strip_suffix(" NOT NULL").unwrap_or(t)
}

/// The normal content of a tag's text, so that `10:30Z` and `10:30:00Z` meet. An unknown tag keeps
/// its text.
fn normal(tag: &str, text: &str) -> Result<String, String> {
    match interpret_tag(tag, text) {
        Ok(Some(i)) => Ok(i.normal_content),
        Ok(None) => Ok(text.to_string()),
        Err(d) => Err(format!("{tag}`{text}`: {}", d.message)),
    }
}

fn tagged(tag: &str, text: &str) -> Result<Value, String> {
    Value::tagged(tag, normal(tag, text)?).map_err(|d| d.message)
}

/// The expected side: a value read from `expected.gram`.
pub fn expected_value(v: &PV) -> Result<Value, String> {
    match v {
        PV::VInteger(i) => Ok(Value::integer(*i)),
        PV::VDecimal(f) => Value::decimal(*f).map_err(|d| d.message),
        PV::VBoolean(b) => Ok(Value::boolean(*b)),
        PV::VString(s) | PV::VSymbol(s) => Ok(Value::string(s.clone())),
        PV::VTaggedString { tag, content } => tagged(tag, content),
        PV::VArray(items) => {
            let converted = items
                .iter()
                .map(expected_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::array(converted))
        }
        PV::VMap(_) => Err("an expected graph holds no map value".into()),
        PV::VRange(_) => Err("an expected graph holds no range value".into()),
        PV::VMeasurement { .. } => Err("an expected graph holds no measurement value".into()),
        other => Err(format!("an expected graph holds no value like {other:?}")),
    }
}

fn tag_of(value_type: &str) -> Option<&'static str> {
    Some(match value_type {
        "DATE" => "date",
        "ZONED TIME" => "time",
        "LOCAL TIME" => "localtime",
        "ZONED DATETIME" => "datetime",
        "LOCAL DATETIME" => "localdatetime",
        "DURATION" => "duration",
        "UUID" => "uuid",
        _ => return None,
    })
}

/// The database side for a scalar, or a vector, from its `valueType()` and `toString()`.
pub fn database_scalar(value_type: &str, text: &str) -> Result<Value, String> {
    let t = strip_not_null(value_type);
    match t {
        "INTEGER" => text
            .parse::<i64>()
            .map(Value::integer)
            .map_err(|e| format!("integer `{text}`: {e}")),
        "FLOAT" => {
            let f: f64 = text.parse().map_err(|e| format!("float `{text}`: {e}"))?;
            Value::decimal(f).map_err(|d| d.message)
        }
        "BOOLEAN" => Ok(Value::boolean(text == "true")),
        "STRING" => Ok(Value::string(text)),
        _ => {
            if let Some(tag) = tag_of(t) {
                return tagged(tag, text);
            }
            if let Some(kind) = t.strip_prefix("VECTOR<") {
                return database_vector(kind, text);
            }
            Err(format!("no value table entry for the type `{value_type}`"))
        }
    }
}

fn database_vector(kind_and_dim: &str, text: &str) -> Result<Value, String> {
    let kind = kind_and_dim.split(' ').next().unwrap_or("");
    let dtype = match kind {
        "FLOAT32" => EmbeddingDtype::F32,
        "FLOAT" => EmbeddingDtype::F64,
        "FLOAT16" => EmbeddingDtype::F16,
        "BFLOAT16" => EmbeddingDtype::BF16,
        other => return Err(format!("a vector of kind {other} has no gram form")),
    };
    let inner = text
        .strip_prefix("vector([")
        .and_then(|t| t.split("], ").next())
        .ok_or_else(|| format!("cannot read the vector `{text}`"))?;
    let floats = inner
        .split(", ")
        .map(|e| {
            e.parse::<f64>()
                .map_err(|err| format!("vector element `{e}`: {err}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bytes = relate_gram::__test_support::narrow(dtype, &floats);
    let embedding = Embedding::new(dtype, floats.len(), bytes).map_err(|d| d.message)?;
    Ok(embedding.to_value())
}

/// A point from its parts, read with `.srid`, `.x`, `.y` and `.z`.
pub fn database_point(srid: i64, x: f64, y: f64, z: Option<f64>) -> Result<Value, String> {
    let text = match z {
        Some(z) => format!("SRID={srid};{x} {y} {z}"),
        None => format!("SRID={srid};{x} {y}"),
    };
    tagged("point", &text)
}

/// A list from the element type of its `valueType()` and the `toString()` of each element.
pub fn database_list(value_type: &str, items: &[String]) -> Result<Value, String> {
    let t = strip_not_null(value_type);
    let inner = t
        .strip_prefix("LIST<")
        .and_then(|r| r.strip_suffix('>'))
        .ok_or_else(|| format!("not a list type: `{value_type}`"))?;
    if inner == "NOTHING" {
        return Ok(Value::array(Vec::new()));
    }
    let element = strip_not_null(inner);
    let converted = items
        .iter()
        .map(|i| database_scalar(element, i))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::array(converted))
}
