//! The consumer test: can relate-gram's front half be rebuilt as a policy over the typed layer?
//!
//! `policy` below is Relate's strict-subset rules written against `gramsyn` only (no tree walk).
//! It is run over every invalid fixture of relate-gram and compared with the fixture's `.expect`
//! file: the same code at the same line, column and byte offset.
use gram_value::{Kind, Value};
use gramsyn::*;
use std::collections::BTreeMap;

type Found = (String, usize, usize, usize);

fn at(d: &Document, code: &str, s: Span) -> Found {
    let (l, c) = d.line_col(s.start);
    (code.to_string(), l, c, s.start)
}

fn label_rules(d: &Document, labels: &[Label], out: &mut Vec<Found>) {
    for l in labels {
        if l.double_colon {
            out.push(at(d, "construct.double-colon", l.colon_span));
        }
    }
}

fn record_rules(d: &Document, r: &Record, header: bool, out: &mut Vec<Found>) {
    for p in &r.properties {
        if p.double_colon {
            out.push(at(d, "construct.double-colon", p.colon_span));
        }
        if !header {
            if p.key.text.is_empty() {
                out.push(at(d, "key.empty", p.key.span));
            } else if p.key.text.starts_with('@') {
                out.push(at(d, "key.reserved", p.key.span));
            }
        }
        value_rules(d, &p.value, out);
    }
}

fn kind_key(v: &Value) -> String {
    match v {
        Value::Tagged(t) => format!("tag:{}", t.tag()),
        other => format!("{:?}", other.kind()),
    }
}

fn value_rules(d: &Document, l: &Lowered, out: &mut Vec<Found>) {
    match l.node_kind {
        "range" => out.push(at(d, "value.range", l.span)),
        "measurement" => out.push(at(d, "value.measurement", l.span)),
        "map" => out.push(at(d, "value.map", l.span)),
        "octal" => out.push(at(d, "value.octal", l.span)),
        "integer" if l.value.kind() == Kind::BigInteger => {
            out.push(at(d, "value.integer-range", l.span))
        }
        "decimal" => {
            let digits = d.text(l.span);
            let f: f64 = match &l.value {
                Value::BigDecimal(b) => b.as_str().parse().unwrap_or(f64::INFINITY),
                _ => 1.0,
            };
            let nonzero = digits.chars().any(|c| ('1'..='9').contains(&c));
            if !f.is_finite() || (f == 0.0 && nonzero) {
                out.push(at(d, "value.decimal", l.span));
            }
        }
        "tagged_string" => {
            if let Value::Tagged(t) = &l.value {
                if let Some(tag) = relate_gram::typed::Tag::from_name(t.tag()) {
                    if tag.validate(t.content()).is_err() {
                        out.push(at(d, "value.tag-content", l.span));
                    }
                }
            }
        }
        "array" => {
            // Relate stores no list of vectors (ADR-014): an array of embeddings.
            if let Value::Array(items) = &l.value {
                let all_embeddings = !items.is_empty()
                    && items.iter().all(|v| matches!(v, Value::Tagged(t) if t.tag() == "embedding"));
                if all_embeddings {
                    out.push(at(d, "value.tag-content", l.span));
                    return;
                }
            }
            if let Value::Array(items) = &l.value {
                if let Some(first) = items.first() {
                    let want = kind_key(first);
                    if let Some(i) = items.iter().position(|v| kind_key(v) != want) {
                        out.push(at(d, "value.array-mixed", l.parts[i]));
                    }
                }
            }
        }
        _ => {}
    }
}

fn subject_rules(d: &Document, s: &Option<Subject>, out: &mut Vec<Found>) {
    if let Some(s) = s {
        label_rules(d, &s.labels, out);
        if let Some(r) = &s.record {
            record_rules(d, r, false, out);
        }
    }
}

fn element_rules(d: &Document, e: &Element, out: &mut Vec<Found>) {
    match e {
        Element::Node { subject, .. } => subject_rules(d, subject, out),
        Element::Reference { .. } => {}
        Element::SubjectPattern { span, .. } => out.push(at(d, "construct.subject-pattern", *span)),
        Element::Annotated { span, .. } => out.push(at(d, "construct.annotation", *span)),
        Element::Relationship {
            left, arrow, right, ..
        } => {
            element_rules(d, left, out);
            if arrow.family != ArrowFamily::Dash {
                out.push(at(d, "rel.arrow-family", arrow.span));
            }
            match arrow.direction {
                Direction::Undirected => out.push(at(d, "rel.undirected", arrow.span)),
                Direction::Both => out.push(at(d, "rel.bidirectional", arrow.span)),
                _ => {}
            }
            // A missing type is reported only for an arrow that is otherwise fine.
            let plain = arrow.family == ArrowFamily::Dash
                && matches!(arrow.direction, Direction::Right | Direction::Left);
            let labels = arrow.subject.as_ref().map(|s| s.labels.len()).unwrap_or(0);
            match labels {
                0 if plain => out.push(at(d, "rel.no-type", arrow.span)),
                0 | 1 => {}
                _ => out.push(at(d, "rel.many-labels", arrow.span)),
            }
            subject_rules(d, &arrow.subject, out);
            element_rules(d, right, out);
        }
    }
}

fn policy(d: &Document) -> Vec<Found> {
    let mut out = Vec::new();
    // Syntax errors stop everything else, as in relate-gram.
    for x in &d.diagnostics {
        match x.code {
            "syntax-error" => out.push(at(d, "parse.error", x.span)),
            "missing-node" => out.push(at(d, "parse.missing", x.span)),
            _ => {}
        }
    }
    if !out.is_empty() {
        return out;
    }
    for x in &d.diagnostics {
        match (x.code, x.node_kind) {
            ("duplicate-key", _) => out.push(at(d, "key.duplicate", x.span)),
            ("out-of-range", "hexadecimal" | "octal") => out.push(at(d, "value.integer-range", x.span)),
            _ => {}
        }
    }
    if let Some(r) = &d.record {
        record_rules(d, r, true, &mut out);
        for p in &r.properties {
            if p.key.text == "kind" {
                if let Value::String(s) = &p.value.value {
                    if s.as_str() != "property-graph" {
                        out.push(at(d, "file.kind", p.value.span));
                    }
                }
            }
        }
    }
    for e in &d.elements {
        element_rules(d, e, &mut out);
    }
    let (_, projected) = project(d);
    for x in projected {
        if x.code == "duplicate-definition" {
            out.push(at(d, "id.duplicate-definition", x.span));
        }
    }
    out.sort();
    out
}

fn expected(path: &std::path::Path) -> Vec<Found> {
    let text = std::fs::read_to_string(path.with_extension("expect")).unwrap();
    let mut v: Vec<Found> = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let p: Vec<&str> = l.split_whitespace().collect();
            (p[0].to_string(), p[1].parse().unwrap(), p[2].parse().unwrap(), p[3].parse().unwrap())
        })
        .collect();
    v.sort();
    v
}

#[test]
fn the_typed_layer_reproduces_relate_grams_rejections() {
    let dir = format!("{}/../../../../tests/fixtures/invalid", env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "gram"))
        .collect();
    paths.sort();
    // codes that need a database, a byte-level encoding check, or the identity planner
    let out_of_scope = ["id.kind-conflict", "id.endpoints-differ", "id.rel-never-defined", "input.encoding"];
    let (mut same, mut skipped, mut differ) = (0, 0, Vec::new());
    let mut by_code: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for p in &paths {
        let want = expected(p);
        if want.iter().any(|w| out_of_scope.contains(&w.0.as_str())) {
            skipped += 1;
            continue;
        }
        let bytes = std::fs::read(p).unwrap();
        let Ok(text) = String::from_utf8(bytes) else {
            skipped += 1;
            continue;
        };
        let got = policy(&parse(&text));
        let ok = got == want;
        for w in &want {
            let e = by_code.entry(w.0.clone()).or_default();
            e.1 += 1;
            if ok {
                e.0 += 1;
            }
        }
        if ok {
            same += 1;
        } else {
            differ.push(format!("{}\n    want {:?}\n    got  {:?}", p.file_name().unwrap().to_string_lossy(), want, got));
        }
    }
    println!("reproduced {same}, out of scope {skipped}, different {}", differ.len());
    for (c, (ok, n)) in &by_code {
        println!("  {c}: {ok}/{n}");
    }
    for d in &differ {
        println!("{d}");
    }
    assert!(differ.is_empty(), "{} fixtures differ", differ.len());
}
