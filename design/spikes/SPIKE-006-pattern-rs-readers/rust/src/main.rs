//! Spike: read each input with relateby-gram and print one canonical description per input,
//! so the output can be diffed against the TypeScript and Python probes.
use gram_codec::parse_gram_with_header;
use pattern_core::{Pattern, Subject, Value};
use std::collections::BTreeMap;

fn value(v: &Value) -> String {
    match v {
        Value::VInteger(i) => format!("int:{i}"),
        Value::VDecimal(d) => format!("dec:{d:?}"),
        Value::VBoolean(b) => format!("bool:{b}"),
        Value::VString(s) => format!("str:{s:?}"),
        Value::VSymbol(s) => format!("sym:{s}"),
        Value::VTaggedString { tag, content } => format!("tag:{tag}:{content:?}"),
        Value::VArray(a) => format!("[{}]", a.iter().map(value).collect::<Vec<_>>().join(", ")),
        Value::VMap(m) => map(&m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        other => format!("other:{other:?}"),
    }
}

fn map(m: &BTreeMap<String, Value>) -> String {
    let inner: Vec<String> = m.iter().map(|(k, v)| format!("{k}={}", value(v))).collect();
    format!("{{{}}}", inner.join(", "))
}

fn pattern(p: &Pattern<Subject>, depth: usize) -> String {
    let s = &p.value;
    let mut labels: Vec<&String> = s.labels.iter().collect();
    labels.sort();
    let props: BTreeMap<String, Value> = s
        .properties
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut out = format!(
        "{}pattern id={:?} labels={:?} props={}\n",
        "  ".repeat(depth),
        s.identity.0,
        labels,
        map(&props)
    );
    for e in &p.elements {
        out.push_str(&pattern(e, depth + 1));
    }
    out
}

fn main() {
    let dir = std::env::args().nth(1).expect("inputs dir");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for path in names {
        let text = std::fs::read_to_string(&path).unwrap();
        println!("== {}", path.file_name().unwrap().to_string_lossy());
        match parse_gram_with_header(&text) {
            Err(e) => println!("error: {}", e.to_string().lines().next().unwrap_or("")),
            Ok((header, patterns)) => {
                match header {
                    Some(h) => {
                        let h: BTreeMap<String, Value> =
                            h.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                        println!("header: {}", map(&h));
                    }
                    None => println!("header: none"),
                }
                for p in &patterns {
                    print!("{}", pattern(p, 0));
                }
            }
        }
    }
}
