const fs=require("fs"),path=require("path");
const {probe}=require(path.resolve(process.argv[2]));
const t=fs.readFileSync(process.argv[3],"utf8");
probe(t);const s=process.hrtime.bigint();const r=probe(t);
console.log("wasm parse+walk",Number(process.hrtime.bigint()-s)/1e6,"ms",r);
