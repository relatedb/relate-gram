// Spike: read each input with @relateby/gram and print the same canonical description as the Rust probe.
import fs from "node:fs"
import path from "node:path"
import { Gram, init } from "@relateby/gram"

const dir = process.argv[2]

function value(v) {
  switch (v._tag) {
    case "IntVal": return `int:${v.value}`
    case "FloatVal": return `dec:${Number.isInteger(v.value) ? v.value.toFixed(1) : v.value}`
    case "BoolVal": return `bool:${v.value}`
    case "StringVal": return `str:${JSON.stringify(v.value)}`
    case "SymbolVal": return `sym:${v.value}`
    case "TaggedStringVal": return `tag:${v.tag}:${JSON.stringify(v.content)}`
    case "ArrayVal": return `[${v.items.map(value).join(", ")}]`
    case "MapVal": return map(v.entries)
    case "NullVal": return "null"
    default: return `other:${JSON.stringify(v)}`
  }
}
function map(entries) {
  return `{${Object.keys(entries).sort().map((k) => `${k}=${value(entries[k])}`).join(", ")}}`
}
function plain(x) {
  if (x === null) return "null"
  if (Array.isArray(x)) return `[${x.map(plain).join(", ")}]`
  switch (typeof x) {
    case "number": return `num:${x}`
    case "string": return `str:${JSON.stringify(x)}`
    case "boolean": return `bool:${x}`
    case "object": return `{${Object.keys(x).sort().map((k) => `${k}=${plain(x[k])}`).join(", ")}}`
  }
  return `other:${String(x)}`
}
function pattern(p, depth) {
  const s = p.value
  let out = `${"  ".repeat(depth)}pattern id=${JSON.stringify(s.identity)} labels=${JSON.stringify([...s.labels].sort())} props=${map(s.properties)}\n`
  for (const e of p.elements) out += pattern(e, depth + 1)
  return out
}

await init()
for (const name of fs.readdirSync(dir).sort()) {
  const text = fs.readFileSync(path.join(dir, name), "utf8")
  console.log(`== ${name}`)
  try {
    const { header, patterns } = await Gram.parseWithHeader(text)
    console.log(header === undefined ? "header: none" : `header: ${plain(header)}`)
    for (const p of patterns) process.stdout.write(pattern(p, 0))
  } catch (e) {
    console.log(`error: ${String(e.message ?? e).split("\n")[0]}`)
  }
}
