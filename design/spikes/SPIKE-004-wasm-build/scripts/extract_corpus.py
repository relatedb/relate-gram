"""Write each input of the tree-sitter-gram corpus to OUT/<file>-<n>.gram."""
import re, sys, pathlib
src, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
count = 0
for f in sorted(src.glob("*.txt")):
    text = f.read_text()
    cases = re.split(r"^={3,}\n.*\n={3,}\n", text, flags=re.M)[1:]
    for i, case in enumerate(cases):
        body = re.split(r"^-{3,}\n", case, maxsplit=1, flags=re.M)[0]
        (out / f"{f.stem}-{i:03}.gram").write_text(body)
        count += 1
print(count, "inputs")
