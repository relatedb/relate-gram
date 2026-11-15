"""Spike: read each input with relateby.gram and print the same canonical description as the Rust probe."""
import json
import pathlib
import sys

import relateby.gram as gram
from relateby.pattern import (ArrayVal, BoolVal, FloatVal, IntVal, MapVal, NullVal,
                              StringVal, SymbolVal, TaggedStringVal)


def value(v):
    if isinstance(v, IntVal):
        return f"int:{v.value}"
    if isinstance(v, FloatVal):
        return f"dec:{v.value!r}"
    if isinstance(v, BoolVal):
        return f"bool:{str(v.value).lower()}"
    if isinstance(v, StringVal):
        return f"str:{json.dumps(v.value, ensure_ascii=False)}"
    if isinstance(v, SymbolVal):
        return f"sym:{v.value}"
    if isinstance(v, TaggedStringVal):
        return f"tag:{v.tag}:{json.dumps(v.content, ensure_ascii=False)}"
    if isinstance(v, ArrayVal):
        return "[" + ", ".join(value(i) for i in v.items) + "]"
    if isinstance(v, MapVal):
        return mapping(v.entries)
    if isinstance(v, NullVal):
        return "null"
    return f"other:{v!r}"


def mapping(entries):
    return "{" + ", ".join(f"{k}={value(entries[k])}" for k in sorted(entries)) + "}"


def plain(x):
    if x is None:
        return "null"
    if isinstance(x, bool):
        return f"bool:{str(x).lower()}"
    if isinstance(x, int):
        return f"int:{x}"
    if isinstance(x, float):
        return f"dec:{x!r}"
    if isinstance(x, str):
        return f"str:{json.dumps(x, ensure_ascii=False)}"
    if isinstance(x, list):
        return "[" + ", ".join(plain(i) for i in x) + "]"
    if isinstance(x, dict):
        return "{" + ", ".join(f"{k}={plain(x[k])}" for k in sorted(x)) + "}"
    return f"other:{x!r}"


def pattern(p, depth):
    s = p.value
    ident = json.dumps(s.identity, ensure_ascii=False)
    labels = json.dumps(sorted(s.labels), ensure_ascii=False)
    out = f'{"  " * depth}pattern id={ident} labels={labels} props={mapping(s.properties)}\n'
    for e in p.elements:
        out += pattern(e, depth + 1)
    return out


for path in sorted(pathlib.Path(sys.argv[1]).iterdir()):
    print(f"== {path.name}")
    try:
        header, patterns = gram.parse_with_header(path.read_text(encoding="utf-8"))
        print("header: none" if header is None else f"header: {plain(header)}")
        for p in patterns:
            sys.stdout.write(pattern(p, 0))
    except Exception as e:  # noqa: BLE001
        print(f"error: {str(e).splitlines()[0] if str(e) else type(e).__name__}")
