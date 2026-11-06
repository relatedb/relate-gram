# Items moved out of RFC-001

Two items were listed as open questions in RFC-001 (gram graph support for Relate) during review. Neither is a question about the design, so they are kept here as planning notes. They bind nothing.

## Updating the gram parser

`gram-codec` (in pattern-rs) is the intended parser. Current tree-sitter-gram supports scalar arrays inside property maps, and the codec may lag the grammar. Open: which parts of the codec need updating to match the current grammar, and whether that work happens before or alongside the loader. tree-sitter-gram's test corpus is the conformance reference.

## Candidate follow-on RFCs

- Identity and namespaces (load-time resolution, merge rules, `@id`, managed multi-graph loading).
- The meta node.
- CLI integration.
- Language bindings (TypeScript, Python) with client-side result structures. Deferred until the Rust core and CLI exist.

Value mapping is RFC-002. ADRs for the meta node format and the namespace format come after their parent RFCs.
