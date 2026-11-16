// Run the wasm probe on every file given; prints "<file> <nodes> <errors>".
const fs = require("fs"), path = require("path");
const { probe } = require(path.resolve(process.argv[2]));
for (const f of process.argv.slice(3)) {
  const text = Buffer.from(fs.readFileSync(f)).toString("utf8");
  console.log(`${path.basename(f)} ${probe(text)}`);
}
