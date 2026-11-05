# S1: Render host for MathJax and Mermaid

Plan task 1.1. DESIGN §6.1, §6.2, §6.4, §6.5, §6.8, §13 (S1), §14 (UNVERIFIED items).

Measured on Linux (x86_64, Chrome for Testing 153.0.8010.12 as a proxy, Node 22.23.4, Rust 1.97.0). **Nothing here ran on WebKit or macOS.** Every WebKit-specific question stays open until a macOS run.

## Pinned versions

| Component | Version | Notes |
|-----------|---------|-------|
| mermaid.js | **12.3.0** | npm `mermaid@12.3.0`. Tested with the `mermaid.esm.min.mjs` entry. |
| MathJax v3 | **3.2.2** (`mathjax-full@3.2.2`) | The last v3 release. npm now marks it deprecated in favour of the v4 scoped package `@mathjax/src`. DESIGN says v3, so v3.2.2 is pinned; moving to v4 is a separate decision. |
| rquickjs | 0.14.0 (QuickJS-NG) | Prototype engine. |
| resvg | 0.46.0 | Rasterizer. 0.49.0 is current; 0.46.0 was pinned for this spike. |
| esbuild | 0.28.2 | Build-time only, to bundle MathJax into one script. |

Nothing from `node_modules` or the bundle is committed.

## Options compared

| Option | Runs Mermaid | Runs MathJax | Result |
|--------|--------------|--------------|--------|
| A. Hidden WKWebView (`wry`, `with_visible(false)`) | Yes (DOM) | Yes | Required for Mermaid. **Not tested here**, because it needs macOS. Only Chrome was run as a proxy (see below). |
| B. Offscreen NSWindow with a WKWebView | Yes (DOM) | Yes | Fallback in DESIGN §6.8. Not tested (needs macOS). |
| C. Embedded QuickJS (rquickjs) + MathJax with liteAdaptor | No (needs a DOM and SVG layout) | **Yes**, works without a DOM | **Prototyped and works.** |
| D. Embedded V8 (deno_core) | Unknown | Likely | Not tried. A V8 build is far heavier than QuickJS for one formula. |
| E. Pure-Rust engine (boa) | No (needs a DOM) | Unknown | Not tried. |
| F. Bundled Node sidecar | Needs jsdom plus SVG layout shims, not tried | Yes | MathJax measured in Node with no DOM (see below). The Node binary is 125 MB, so this costs a lot of app size. |

Mermaid needs a DOM with SVG layout, so C cannot run it. D, E and F would need DOM shims for layout; none was tried. `getBBox` is called across many of its diagram chunks (for example `chunk-AXFCVDMN.mjs` has 33 calls), so the shims would have to model real text metrics.

## Acceptance checks

| Acceptance item (plan 1.1) | Result |
|----------------------------|--------|
| Measured round-trip latency | Measured, but not the host round trip. In-process QuickJS per formula: 1.6 to 8.2 ms (first call), 8.6 ms warm mean. Chrome, Mermaid warm render: 30 to 42 ms. The WKWebView round trip is **not measured** (macOS). |
| Mermaid output with `htmlLabels: false` has no `<foreignObject>` | **Yes, in Chrome.** Flowchart, default and dark themes: 0 `<foreignObject>`. Control with `htmlLabels: true`: 6. The check covers flowcharts only. Other diagram types are not checked. |
| resvg renders the output | **Yes.** MathJax output (fraction, sum, inline, error case) and the Mermaid flowchart all rasterize with visible glyphs and edges. Checked by eye on the sum formula and the flowchart. The Mermaid text uses system fonts on Linux; DESIGN §6.5 requires bundled fonts, which is still to do. |
| Go/no-go line | Below. |
| Names the pinned mermaid.js and MathJax v3 versions | mermaid 12.3.0, MathJax v3 3.2.2 (table above). |

## Mermaid in Chrome (proxy for WebKit, not WebKit)

Harness: `mermaid.esm.min.mjs` from the pinned package, served from a local static server, driven by `puppeteer-core` against `/usr/bin/google-chrome`. Source: a four-node flowchart with two labelled edges and one back edge. Each config was rendered 11 times. The first render is the cold figure.

| Config | SVG bytes | `<foreignObject>` | `<text>` | Cold render | Warm median (10 runs) |
|--------|-----------|-------------------|----------|-------------|-----------------------|
| `htmlLabels: false`, theme `default` | 16,209 | **0** | 6 | 497 ms (includes lazy chunk load) | 41.7 ms |
| `htmlLabels: false`, theme `dark` | 16,482 | **0** | 6 | 35.3 ms | 34.3 ms |
| control: `htmlLabels: true`, theme `default` | 15,670 | 6 | 0 | 31.2 ms | 29.5 ms |

The default and dark outputs differ, so the theme mapping (DESIGN §6.3) changes the SVG. The control confirms that the `foreignObject` count can detect HTML labels.

## MathJax without a DOM

Bundle: `mathjax-entry.mjs` (below) built with esbuild:

```text
node_modules/.bin/esbuild mathjax-entry.mjs --bundle --minify --format=iife --platform=neutral --target=es2019 --outfile=mathjax-bundle.js
```

```js
// mathjax-entry.mjs: MathJax v3 TeX -> SVG with no DOM (liteAdaptor, fontCache 'none').
import { mathjax } from 'mathjax-full/js/mathjax.js';
import { TeX } from 'mathjax-full/js/input/tex.js';
import { SVG } from 'mathjax-full/js/output/svg.js';
import { liteAdaptor } from 'mathjax-full/js/adaptors/liteAdaptor.js';
import { RegisterHTMLHandler } from 'mathjax-full/js/handlers/html.js';
import { AllPackages } from 'mathjax-full/js/input/tex/AllPackages.js';

const adaptor = liteAdaptor({ fontSize: 16 });
RegisterHTMLHandler(adaptor);
const tex = new TeX({ packages: AllPackages });
const svg = new SVG({ fontCache: 'none' });
const doc = mathjax.document('', { InputJax: tex, OutputJax: svg });

globalThis.texToSvg = function (src, display) {
  const node = doc.convert(src, { display: display });
  return adaptor.outerHTML(node);
};
```

Results:

- Bundle: 1,820,046 bytes minified, 616,201 bytes gzipped. It includes all TeX packages (`AllPackages`). A `base` plus `ams` subset would be smaller. Not measured.
- Node 22 (V8, no DOM): init 106 ms, first conversion 10.6 ms, warm mean 4.8 ms (200 runs of a sum formula). Output has no `<foreignObject>`, uses `currentColor` for the fill, and error input sets `data-mjx-error`.
- rquickjs 0.14.0 (QuickJS-NG), prototype in the next section: init 110 ms (runtime plus bundle eval). Per-formula first call: `\frac{a}{b}` 1.7 ms, the sum formula 8.2 ms, inline `x^2+y^2=r^2` 4.0 ms, error input 1.6 ms. Warm mean for the sum formula: 8.6 ms (200 runs).

The output contains a `<mjx-container>` wrapper, so the host must extract the `<svg>` element. Theme colour is applied by replacing `currentColor` with the theme's token (the prototype does this with `#1d1d1f` light and `#f5f5f7` dark).

## Prototype (feature-gated example)

- `crates/md_render/examples/spike_s1_math.rs`: loads the bundle into a QuickJS context, converts four formulas (including one error case), times the conversions, writes light and dark SVGs, and rasterizes with resvg at 2x. It can also rasterize a Mermaid SVG passed as a third argument.
- `crates/md_render/Cargo.toml`: optional deps `rquickjs` and `resvg`, features `spike-quickjs` and `spike-resvg`. Off by default. Not wired into `RenderQueue` (`queue.rs` is unchanged).

Run:

```text
cargo run -p md_render --example spike_s1_math --features "spike-quickjs spike-resvg" --release -- <mathjax-bundle.js> <out-dir> [mermaid.svg]
```

Output from the run: all four MathJax formulas converted; 0 `<foreignObject>` in each; error case flagged `data-mjx-error=true`; resvg painted pixels for every output; the Mermaid SVG rasterized to 720x816 px at 2x with its labels and arrows intact.

Without the features the example compiles to a stub that prints the usage line, so the workspace checks do not build rquickjs or resvg.

### Binary size (release, `spike_s1_math`)

| Build | Unstripped | Stripped |
|-------|------------|----------|
| No features (baseline stub) | 445,544 B | 342,416 B |
| `spike-quickjs` (rquickjs, QuickJS-NG) | 1,825,816 B (+1.38 MB) | 1,609,032 B (+1.27 MB) |
| `spike-quickjs spike-resvg` | 5,479,640 B (+5.03 MB) | 4,703,048 B (+4.36 MB) |

The QuickJS cost is the engine. The resvg cost is needed anyway for DESIGN §6.5, so it is not charged to math. The MathJax bundle is a separate 1.82 MB file, or 616 KB gzipped. It is not in these figures; in the app it would be embedded or loaded from the bundle.

## Recommendation for tasks 6.1, 6.3 and 7.1

Split the render host by engine. The current DESIGN §6.1 puts both engines in one WKWebView, but math does not need WebKit.

- **6.3 MathRenderer:** implement it in process on rquickjs, using the bundle above. Hold the context in a worker thread. `RenderBackend` requires `Send + Sync`. The rquickjs 0.14.0 source marks `Runtime` as `Send + Sync` and `Context` as `Send` only, so wrap the context in a `Mutex` (not verified under concurrent load). Use the synchronous `doc.convert`; the plan's `tex2svgPromise` name is a deviation, but the output is the same. Commit the pinned bundle under `md_render/assets/mathjax/` only if it is needed in the repo. Otherwise build it in CI. Also consider the subset (`base` plus `ams`) to cut the 1.8 MB.
- **6.1 Render host service:** scope it to Mermaid only. The WKWebView host is still needed for 7.1 and its acceptance (a fixed test string returns SVG, runs off the UI thread, starts and stops cleanly) still applies. Its layout behaviour stays UNVERIFIED until a macOS run.
- **7.1 DiagramRenderer with Mermaid:** keep `htmlLabels: false` at the top level. Use the WKWebView path. Check `<foreignObject>` absence on every diagram type, not just flowcharts. Measure the real WKWebView round trip on macOS before choosing the debounce (DESIGN §16 item 8 is still open). The mermaid dist is 123 MB across chunks, so measure the bundle that actually ships.
- **DESIGN follow-up:** §6.1 and §6.8 should be amended so that math runs in process. That is a decision for the owner, and this note does not edit DESIGN.

## Not verified

- The hidden WKWebView (layout, timers, thread behaviour, async results) on macOS. DESIGN §14 UNVERIFIED item 1 is still open.
- Whether `htmlLabels: false` removes `<foreignObject>` for diagram types other than flowcharts. DESIGN §14 UNVERIFIED item 3 is partly answered (flowcharts only).
- resvg and `<foreignObject>`: moot for these outputs, because neither MathJax nor Mermaid (with `htmlLabels: false`) emits any. Whether resvg handles `foreignObject` for other inputs was not tested. DESIGN §14 UNVERIFIED item 2 is still open for that case.
- Latency through the real host (thread hop, WebKit, IPC). Only in-process and Chrome numbers exist.
- deno_core, boa and a Node sidecar with real IPC were not run.
- The MathJax bundle in the app (size, embedding, licence notices for the bundled JS) and the `base`/`ams` subset.

Go/no-go: GO (split host). MathJax runs in embedded QuickJS with no DOM, and its output is SVG with no `<foreignObject>`, so 6.3 can go ahead without a WebView. Mermaid needs a DOM, so 7.1 depends on the hidden WKWebView host, which is still UNVERIFIED on macOS (DESIGN §14), and that check remains a blocker for 6.1 and 7.1.

## Decision

The owner chose the split host and MathJax v4. This section records that decision. DESIGN §6.1, §6.4 and §6.8 are amended to match.

- **Split host.** Math runs in process: MathJax in embedded QuickJS (rquickjs) with the liteAdaptor, rasterized by resvg (`md_render::raster`). A hidden WebView runs Mermaid only, because Mermaid needs a DOM. Option C is the math path, and option A is the Mermaid path.
- **MathJax v4.** Bundle the v4 `@mathjax/src` package instead of v3 `mathjax-full`. The MathJax v3.2.2 pin in the table above is superseded for the app. The v3 numbers in this note are measurements of v3 only and do not carry over to v4.
- **Verification gap.** The QuickJS path was prototyped with v3 only. It must be re-verified with v4 in task 6.3 (bundle, output shape, error handling, and the v4 call that replaces `tex2svgPromise`).
- **Still open.** Hidden WKWebView or offscreen NSWindow, WebKit layout behaviour, and the macOS round-trip measurement (DESIGN §14). These gate Mermaid only (tasks 6.1 and 7.1).
- **Unchanged.** mermaid.js 12.3.0 remains the pinned Mermaid version.
