---
id: SPIKE-004
title: Building the Gram Parser for WebAssembly
status: done
rfcs: [RFC-005, RFC-006]
created: 2026-11-13
---

# SPIKE-004: Building the Gram Parser for WebAssembly

## 1. Question

The WebAssembly question of two RFCs: RFC-005 (relate-gram in other languages) 7.2, how the parser reaches a browser page, and RFC-006 (a graph data model for the gram subset), which needs the same parser client-side. (The spike was written on 2026-11-13 against an earlier draft that was also numbered RFC-005, as a graph data model; that draft is now RFC-006.) Can `tree-sitter` 0.25 with `tree-sitter-gram` 0.6.2 (the parser ADR-007 chose, a C library built by a build script) be built for WebAssembly, and is the result small and fast enough to ship to a browser? If it cannot, what is the cheapest replacement that still agrees with the grammar's corpus?

## 2. Method

A throwaway crate under `scripts/probe` depends only on `tree-sitter` and `tree-sitter-gram` (no `relate`), parses a gram document, and returns the count of nodes and any ERROR or MISSING nodes. Build it for `wasm32-unknown-unknown` and `wasm32-wasip1` with the toolchain the repository pins. Record: whether it builds and what it needs (C toolchain with a wasm target, `wasm-bindgen`), the size of the `.wasm` before and after `wasm-opt`, and whether a Node run parses the corpus inputs with the same result as the native run. Corpus inputs come from `external/tree-sitter-gram`.

## 3. Time-box

Half a day. If neither target builds after a documented attempt with a wasm-capable C toolchain, stop and list the failures.

## 4. Notes

2026-11-13
- `tree-sitter` 0.25 (the C runtime) does not build for `wasm32-unknown-unknown` or `wasm32-wasip1` here: both the runtime and `tree-sitter-gram`'s generated `parser.c` include `stdlib.h`, and neither target has a libc in this toolchain. A wasi-sdk sysroot would supply one; it was not downloaded for the spike.
- Replacing the runtime with `tree-sitter-c2rust` 0.25.2 (the same runtime translated to Rust, a drop-in for the `tree-sitter` API) removes the runtime's libc need. The grammar's `parser.c` then needs only `NULL`, so an empty `stdlib.h` containing `#include <stddef.h>`, passed as `CFLAGS_wasm32_unknown_unknown=-I scripts/probe/shim`, is enough.
- The link first failed with `undefined symbol: tree_sitter_gram`: Apple's `ar` does not index WebAssembly objects. Setting `AR_wasm32_unknown_unknown` to `llvm-ar` (rustup component `llvm-tools`) fixed it.
- `wasm-bindgen` CLI and crate versions must match (0.2.128 pinned).

## 5. Findings

Run with the commands in `scripts/` (`probe` builds the wasm, `native` is the C-runtime reference, `extract_corpus.py`, `run_wasm.js`, `time_wasm.js`).

- **It builds**, with the `tree-sitter-c2rust` runtime and the two build settings in the notes (a stub `stdlib.h`, and `llvm-ar`). The grammar's C file is compiled by `clang` for `wasm32-unknown-unknown`; no wasi-sdk.
- **Size** of the parse-and-walk probe: 230 KB from `wasm-bindgen`, 170 KB after `wasm-opt -Oz`, 67 KB gzipped. A real core with a model and a serializer will be larger; the parser is not the dominant cost.
- **Agreement**: the Node run reports the same node count and the same ERROR/MISSING count as the native C runtime for all 214 inputs (172 extracted from `tree-sitter-gram`'s corpus, 42 project fixtures valid and invalid). Counts are the only comparison; trees were not compared node by node.
- **Speed**: a 5.3 MB file of 100,000 elements (3.1 million nodes) parses and walks in 2.1 s in Node, against 0.37 s total for the native C runtime, about 5.6 times slower. A 10,000-element file is a tenth of that.
- **Not tested**: a browser, a bundler, the C runtime on a wasi sysroot, memory use, and `wasm32` for the real core (only the parser probe).

## 6. Conclusion

The question is answered: yes. The parser can ship to JavaScript and the browser as WebAssembly with the grammar and conformance unchanged, at about 170 KB for the parser alone and several times slower than native. The cost is a pure-Rust runtime (`tree-sitter-c2rust`) in place of the C runtime for that target, and two build workarounds that a packaging ADR would have to carry (a stub header for the grammar's C file and `llvm-ar`). The C runtime on a wasi-sdk sysroot is an alternative to measure if the speed gap matters.

Text for RFC-005 7.2 and RFC-006: "Answered by SPIKE-004: tree-sitter-gram builds to WebAssembly with the pure-Rust runtime, 170 KB for the parser, identical parse results on 214 inputs, about 5.6 times slower than native on a 5 MB file. Browser use and the C runtime on wasi-sdk remain untested."

## 7. Status of the measurements (added 2026-11-16)

The measurements are of `tree-sitter-gram` 0.6.2 with `tree-sitter` 0.25. The grammar is now 0.7.0 and the probe has not been re-run against it, so the size, speed and agreement figures are for 0.6.2. The 214 inputs include 172 cases from that version's corpus. Browser use is still untested.
