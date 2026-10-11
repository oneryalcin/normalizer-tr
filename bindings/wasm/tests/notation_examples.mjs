// Runs the core's notation goldens through the built package: `node tests/notation_examples.mjs`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import init, { Normalizer } from "../pkg/normalizer_tr_wasm.js";

await init({ module_or_path: readFileSync(new URL("../pkg/normalizer_tr_wasm_bg.wasm", import.meta.url)) });
const cases = JSON.parse(
  readFileSync(new URL("../../../tests/fixtures/notation-examples.json", import.meta.url), "utf8"),
);
const normalizer = new Normalizer();
for (const { text, expected } of cases) {
  for (const ambiguityPolicy of ["preserve", "reject", "fallback"]) {
    const result = normalizer.normalize(text, { ambiguityPolicy });
    assert.equal(result.normalizedText, expected, `${text} ${ambiguityPolicy}`);
    assert.ok(result.complete && !result.issues.length && !result.fallbackUsed, text);
  }
}

// Field names and error mapping are binding code, not core code.
const [segment] = normalizer.normalize("25 TL").segments;
assert.deepEqual(segment, { startByte: 0, endByte: 5, kind: "money", text: "yirmi beş Türk lirası", ruleId: "quantity" });
assert.throws(() => normalizer.normalize("x".repeat(40_000)), { name: "NormalizationError", code: "limit_exceeded", limitKind: "input" });
assert.throws(() => normalizer.normalize(1), TypeError);
console.log(`ok: ${cases.length} notation examples in 3 policies`);
