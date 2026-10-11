# normalizer-tr for JavaScript (WebAssembly)

The Rust core compiled to WebAssembly, for browsers and other JavaScript
runtimes. Normalization executes the actual Rust core, so output matches the
Rust and Python APIs.

## Build

Requires `rustup` with the `wasm32-unknown-unknown` target and
[`wasm-pack`](https://wasm-bindgen.github.io/wasm-pack/).

```sh
rustup target add wasm32-unknown-unknown
wasm-pack build --target web --release
```

Run the binding test against the build with `node tests/notation_examples.mjs`.

The package is written to `pkg/`: an ES module, TypeScript types and a
`.wasm` file of about 1.2 MB (about 465 kB gzipped).

## Use

```js
import init, { Normalizer } from "./pkg/normalizer_tr_wasm.js";

await init(); // fetches normalizer_tr_wasm_bg.wasm next to the module
const normalizer = new Normalizer(); // reuse across calls

const result = normalizer.normalize("25 TL; 5 kg");
// result.normalizedText === "yirmi beş Türk lirası; beş kilogram"
// result.complete === true

normalizer.normalize("1.234; AB12", { ambiguityPolicy: "fallback" }).normalizedText;
// "bin iki yüz otuz dört; a be bir iki"
```

In Node, pass the bytes yourself:
`await init({ module_or_path: readFileSync("pkg/normalizer_tr_wasm_bg.wasm") })`.

Options are `ambiguityPolicy` (`"preserve"` by default, `"reject"` or
`"fallback"`) and `hints` (`{ startByte, endByte, kind }`). Result fields are
the Python API's fields in camelCase; kind, category and error code strings
are identical. Failures throw an `Error` named `NormalizationError` with
`code`, `limitKind` and `issues`; wrong argument types throw `TypeError`.

## Differences from the Python API

- All offsets are UTF-8 byte offsets into the original text, not JavaScript
  string indices. Convert with `TextEncoder` when the text is not ASCII.
- No cancellation token or deadline. A call is synchronous and input is
  limited to 32 KiB, so run it in a Web Worker to keep the main thread free.
- A lone surrogate reaches the core as U+FFFD instead of raising
  `invalid_input`. Strip lone surrogates first if the text is untrusted.
