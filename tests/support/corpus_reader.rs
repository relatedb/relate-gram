//! A reader for `tree-sitter test` corpus files.
//!
//! A file holds cases of the form
//!
//! ```text
//! ==================
//! Case name
//! :attribute
//! ==================
//!
//! input
//!
//! ---
//!
//! (expected tree)
//! ```
#![allow(dead_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusCase {
    pub name: String,
    pub attributes: Vec<String>,
    pub input: String,
    pub tree: String,
}

impl CorpusCase {
    /// The corpus expects the input to be a syntax error.
    pub fn is_error(&self) -> bool {
        self.attributes.iter().any(|a| a == ":error")
    }
}

fn is_rule(line: &str, ch: char) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == ch)
}

pub fn parse_corpus(text: &str) -> Vec<CorpusCase> {
    let lines: Vec<&str> = text.lines().collect();
    let mut cases = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if !is_rule(lines[i], '=') {
            i += 1;
            continue;
        }
        // header: rule, name, attributes, rule
        let mut j = i + 1;
        let mut header = Vec::new();
        while j < lines.len() && !is_rule(lines[j], '=') {
            header.push(lines[j]);
            j += 1;
        }
        if j >= lines.len() || header.is_empty() {
            break;
        }
        let name = header[0].trim().to_string();
        let attributes: Vec<String> = header[1..]
            .iter()
            .map(|l| l.trim().to_string())
            .filter(|l| l.starts_with(':'))
            .collect();
        // body: input up to the `---` line, tree up to the next header
        let mut k = j + 1;
        let mut input = Vec::new();
        while k < lines.len() && !is_rule(lines[k], '-') {
            input.push(lines[k]);
            k += 1;
        }
        k += 1;
        let mut tree = Vec::new();
        while k < lines.len() && !is_rule(lines[k], '=') {
            tree.push(lines[k]);
            k += 1;
        }
        cases.push(CorpusCase {
            name,
            attributes,
            input: input.join("\n").trim_matches('\n').to_string(),
            tree: tree.join("\n").trim().to_string(),
        });
        i = k;
    }
    cases
}

/// All corpus files of the submodule, as `(file name, cases)`.
pub fn read_corpus() -> Vec<(String, Vec<CorpusCase>)> {
    let dir = format!(
        "{}/external/tree-sitter-gram/test/corpus",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir}: {e} (run `git submodule update --init`)"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&p).unwrap();
            (name, parse_corpus(&text))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "==================
First case
==================

(a)

---

(gram_pattern
  (node_pattern identifier: (symbol)))

==================
Second case
:error
==================

(a {k: })
(b)

---

(gram_pattern (ERROR))
";

    #[test]
    fn reads_several_cases_from_one_file() {
        let cases = parse_corpus(SAMPLE);
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].name, "First case");
        assert_eq!(cases[0].input, "(a)");
        assert!(cases[0].tree.starts_with("(gram_pattern"));
        assert!(cases[0].tree.ends_with("(symbol)))"));
    }

    #[test]
    fn reads_attributes_and_multiline_input() {
        let cases = parse_corpus(SAMPLE);
        assert_eq!(cases[1].attributes, vec![":error".to_string()]);
        assert!(cases[1].is_error());
        assert!(!cases[0].is_error());
        assert_eq!(cases[1].input, "(a {k: })\n(b)");
    }

    #[test]
    fn the_submodule_corpus_has_cases() {
        let all = read_corpus();
        let total: usize = all.iter().map(|(_, c)| c.len()).sum();
        assert!(total > 150, "found {total} cases");
        assert!(all
            .iter()
            .all(|(_, cases)| cases.iter().all(|c| !c.name.is_empty())));
    }
}
