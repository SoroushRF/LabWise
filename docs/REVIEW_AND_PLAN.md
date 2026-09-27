# LabWise: Review and Rebuild Plan

> Scope: a full review of the code and docs at `d59e729`, and a concrete plan to fix
> what is built and raise its engineering quality. This plan covers **only features that
> already exist or are already claimed**. It adds no new product surface, except where a
> claimed feature can't be made honest without it (each of those is flagged).
>
> This document is the only change in this commit. No code was modified.

---

## 0. Verdict

The idea is good. The build doesn't deliver it yet.

**Idea:** a student drops in a lab handout and, before walking into the lab, gets
(1) the circuit they need to build, (2) the readings they should expect, and (3) the mistakes
that would burn parts. Wokwi and Tinkercad don't do (2) or (3) from a handout, so a
deterministic physics kernel sitting behind an LLM extractor is a real, defensible
differentiator.

**Build:** the codebase is four half-projects that were never joined up:

1. A **Rust MNA kernel**. Its stamping math is correct and it has 38 passing tests. It
   speaks a 7-type schema that nothing else in the app produces.
2. An **LLM extractor**. It speaks a different 26-type, Wokwi-flavoured schema.
3. A **Wokwi renderer**. It draws parts with no simulation behind them. Code is never
   executed, and LEDs are hard-wired to "on".
4. A **Master Plan** that marks several things ✅ that don't work, and claims guarantees
   ("Governance rejects 100% of physically impossible circuits") that are demonstrably false.

The headline defect: **every AI-extracted circuit is rejected by the kernel with a JSON
parse error, 100% of the time.** The frontend sends the wrong field names, the wrong type
vocabulary and the wrong value types (§2, probe #1–2). The "physics validation" panel has
never validated a real extraction.

The fix is not a rewrite of everything. It is:
**one shared contract** (a part catalog plus a canonical circuit IR), **a kernel that
reasons about topology instead of node names**, **a server-owned extraction endpoint with
structured output**, and **a single frontend store**. Most of the existing code
(MNA stamps, Zod, prompt rules, native Wokwi elements, the PDF upload) survives in
refactored form.

---

## 1. Claimed vs actual

| Claim (MASTER_PLAN / code comments) | Reality | Evidence |
|---|---|---|
| "AI extracts → Zod validates → **Rust Governance** validates → Wokwi renders" | Governance **always** rejects AI output, and rendering happens regardless | probe #1, #2; `App.tsx:66-71` renders in parallel, no gate |
| "Governance rejects 100% of physically impossible circuits" | A dead short, an LED straight across 9 V, parallel 9 V‖5 V sources, a reversed LED and a 1 mΩ short all **pass** | probes #3, #4, #5, #9, #11 |
| Stress test 1.2.4 covers "pin conflicts, over-current" | `PhysicsError::{PinConflict, OverVoltage, OverCurrent}` are **never constructed** (dead variants) | `governance.rs:46-61` |
| "Wokwi handles ALL hardware emulation (Arduino code execution…)" | `@wokwi/elements` are **visual web components only**. No AVR emulation exists. LEDs are forced on. | `CircuitBoard.tsx:364-367` |
| Step 4.1.2 ✅ `WokwiEmbed.tsx` | File deleted and replaced by `CircuitBoard.tsx`; docs never updated | git `e9d337e` |
| Step 3.1.5 Refinement loop ❌ | Exists for Zod errors (`extractionPipeline.ts:46-112`), but not for kernel errors | — |
| Golden path marks 3.1.4 / 4.1.1 / 4.1.2 ❌, phase table marks them ✅ | Internal contradiction | `MASTER_PLAN.md` |
| "Gemini 2.0 Flash" | Code defaults to `gemini-2.5-flash-lite`, overridable by any caller | `api/src/index.ts:57` |
| "`components/`, `hooks/`, `CircuitEditorPanel.tsx`, `ExtractionStatus.tsx`" | Do not exist | tree |
| "WASM integrity: 3 tests pass in browser" | Fresh clone **does not typecheck** (`wasm-pkg` missing, and no script builds it) | `tsc -b` fails |
| Kernel header: "before anything is rendered in the **3D projection**"; component library mentions a "York University bookstore kit" | Leftovers from an earlier concept | `lib.rs:15`, `component_library.rs:10` |

---

## 2. Reproduced defects (kernel probe)

I built a scratch harness against `labwise-kernel` (it isn't committed) and fed it the exact
payload shapes the app produces, plus adversarial circuits. Results:

| # | Input | Outcome | Should be |
|---|---|---|---|
| 1 | Exact shape `circuitStore.solve()` sends (`component_type`, `"220"`, `from`/`to`) | ❌ `JSON parse error` | Accept (one contract) |
| 2 | Any Zod-valid type, e.g. `arduino-uno` | ❌ `unknown variant 'arduino-uno'` | Accept; model it or mark it unmodeled |
| 3 | LED directly across 9 V, no resistor | ✅ passes, LED "draws" 90 mA, no finding | **Safety error:** LED over-current |
| 4 | LED **reversed** in series with 330 Ω | ✅ passes, 20.9 mA flows through a reverse-biased LED | LED off, ~0 A |
| 5 | Wire from `vcc_5v` to `gnd` (the prompt's **own** naming convention) | ✅ passes, battery current **−5 × 10⁹ A** | **Short circuit** |
| 6 | `connections: n1→VCC, n2→GND` (the schema's own documented use) | passes governance, solver **singular** | Nets merged, solved |
| 7 | Battery pins listed `negative` first | VCC solves to **−9 V** (polarity from array order, not `pin_id`) | +9 V |
| 8 | Island of resistors not tied to ground | passes governance, solver singular | Warning: floating island, rest solved |
| 9 | 9 V and 5 V sources in parallel | passes governance, solver singular | Error: conflicting voltage sources |
| 10 | Duplicate component id `B1` | passes governance, solver singular (vsource map collision) | Error: duplicate id |
| 11 | 1 mΩ resistor across 9 V | passes, 9 kA | Short / over-power |
| 12 | Resistor pins `"banana"`/`"apple"`, `tolerance: 7.0` | passes | Error: unknown pin; tolerance out of range |
| 13 | Ground net named `"0"` | Solution reports it under key `"GND"` (a key the caller never used) | Report under the caller's net name |
| 14 | `electrical_limits.current_max = 1 mA`, actual 9 mA | passes. `tolerance`/`electrical_limits` are **required but ignored** | Rating violation |

Tooling state at `d59e729`:

- `cargo test`: 38/38 pass. `cargo clippy`: 2 warnings.
- `projection`: `tsc -b` **fails** on a fresh clone. `eslint`: **13 errors** (`no-explicit-any`, refs read during render, `prefer-const`).
- `npm run install-all` at the root is broken. It runs `cd api && … && cd projection`, which resolves to `api/projection`.
- There are no frontend tests, no API tests and no CI.
- No secrets in git history (checked for `AIza…` keys and `.dev.vars`).

---

## 3. Findings by layer

Severity: **S1** = broken, unsafe or false claim · **S2** = incorrect behaviour in realistic use · **S3** = quality, cruft or maintainability.

### A. Architecture and product

- **S1: Four drifting circuit schemas, no single contract.**
  - (a) Rust `Netlist` (7 snake_case types, node-to-node `connections`, required `tolerance`/`electrical_limits`)
  - (b) Zod `ExtractionResult` (26 kebab-case Wokwi types, `"comp:pin"` connections, string values like `"10k"`)
  - (c) TS `types.ts` (a mirror of (a) plus `'arduino'`)
  - (d) `schemas/netlist.schema.json` (a mirror of (a), used by nothing at runtime)

  On top of those, part and pin knowledge is duplicated across five more places: `promptBuilder.ts` (type list and pin rules), `Mapper.ts` (`TYPE_MAP`, `mapPin`), `CircuitBoard.tsx` (`TAG_MAP`, fallback pins), `component_library.rs` (`pin_names`) and `netlist.rs` (the enum). They already disagree. For example, the prompt says Arduino `"GND"`, but `wokwi-arduino-uno` only has `GND.1`/`GND.2`/`GND.3`, so that wire silently doesn't draw.
- **S1: Connectivity is represented twice in the LLM output** (`pins[].node` *and* `connections[]`), and the model has to keep the two consistent. Contradictions are guaranteed, and neither side is cross-checked.
- **S1: The kernel was stranded by a pivot.** The project moved from a "3D projection + physics" concept to an Arduino/Wokwi one. The kernel models analog DC parts. The extractor emits MCU and sensor parts the kernel can't parse. Only `resistor`, `led`, `capacitor`, `battery` and `wire` overlap, and two of those (`capacitor`, `battery`) have no Wokwi element to render.
- **S1: The emulation promise is false** (see §1). No code runs.
- **S2: The planned refinement loop (3.1.5) would conflict with the anti-hallucination rules.** Sending *physics* rejections back to the LLM ("LED has no resistor, fix it") pushes it to invent a resistor the handout never mentions. Only *structural* errors should be auto-repaired. Physics findings belong to the student. That's the whole point of the product.
- **S2: The kernel acts as a gate when it should be an analysis.** A circuit with a mistake is exactly what the student needs to *see*, highlighted. It shouldn't be rejected.
- **S3: The docs are unreliable.** MASTER_PLAN contradicts itself and the tree, there's no root README, and the `projection/` and `api/` READMEs are unedited templates.

### B. Kernel (`kernel/`)

- **S1: Ground, power and short detection are based on node names.** Ground is `{gnd, ground, 0}` (`governance.rs:184`, repeated five times across files). Power is `{vcc, 5v, 3v3, 9v}` (`governance.rs:277`). The prompt tells the LLM to use `vcc_5v`/`vcc_3v3`, so the kernel's own upstream naming bypasses short detection (probe #5). Physics must not depend on string spelling.
- **S1: `connections` are never stamped** (`mna.rs:111-122` adds the nodes but no element), so any netlist that uses them is singular (probe #6).
- **S1: The LED/diode model is a 100 Ω resistor** (`mna.rs:294-304`). It has no forward voltage, so it conducts in reverse (probe #4) and over-current can't be detected (probe #3). LEDs appear in almost every intro lab, so this is a correctness requirement, not the "P2 polish" the plan calls it.
- **S1: No post-solve checks.** The solver computes currents and nobody compares them to ratings. The "smoke test" (2.2.4) is the product's core value and it doesn't exist.
- **S2: Polarity and pin roles come from array order, not `pin_id`** (`mna.rs:289-290`, probe #7). Pin names are never validated against the part (probe #12).
- **S2: The floating-node check counts list entries, not distinct components.** A resistor with both legs on `n1` passes. A floating *island* passes (probe #8). The solver then fails with a generic "singular matrix" that names nothing.
- **S2: No voltage-source loop detection** (parallel sources, source terminals merged), no duplicate-id check (probes #9, #10), and no tolerance range check.
- **S2: Wires are stamped as 1e9 S conductances.** That's ill-conditioned next to 1 MΩ resistors (condition number around 1e15). Wires should *merge nets* (union-find) before stamping.
- **S2: `solve()` always inserts `"GND"`** regardless of the ground's real name (probe #13). LU success isn't checked for conditioning, and there's no residual check.
- **S3: Fake typestate.** `ValidatedNetlist` has `pub` fields, so anyone can construct one. The doc comment says otherwise.
- **S3: Non-deterministic `HashMap` ordering in outputs.** `ComponentType` `Display` duplicates the serde names. Specs use magic numbers (a `0.25 W` resistor, `48 V` max, and so on) with no source. `OpAmp` is in the enum but never stamped or extractable. `electrical_limits` and `tolerance` are required on input but ignored.

### C. WASM bridge (`bridge/`)

- **S1: No solve is exported.** Only `validate_circuit`, `add` and `greet` exist. The last commit ("wire AI extraction to WASM physics engine") reads `result.solution`, which the bridge never returns.
- **S2: Errors are stringly-typed** (`"400 Bad Physics"` plus formatted strings), with no codes and no component/net ids. The UI can't highlight anything, and a repair loop can't consume them.
- **S3:** `add` and `greet` are test cruft in the production API.

### D. AI pipeline (`api/`, `projection/src/synapse/`)

- **S1: Open, cost-bearing relay.** `POST /api/gemini` accepts any prompt and **any `model`** from anyone. CORS doesn't stop `curl`. There's no auth, no rate limit and no size cap. The API key goes in the URL query string (`?key=`), where proxies and logs tend to capture it. Upstream error bodies are returned verbatim.
- **S1: The client owns the system prompt.** The server is a generic LLM proxy, not an extraction service. Moving the prompt server-side fixes both abuse and versioning.
- **S2: No structured output.** Gemini supports a JSON MIME type and a response schema. Today the code regex-strips Markdown fences (`extractJson`) and hopes. Most Zod retries would disappear with native structured output.
- **S2: Zod only checks shape, not meaning.** It never checks that connection endpoints exist, that pins are valid for the part type, that ids are unique, or that the circuit has a ground. Values like `"10k"` and `"4.7u"` are never normalized to SI numbers. The post-filter for pinless components (`schemas.ts:108`) is dead code, because the schema already requires `min(1)` pins.
- **S2: No 429/backoff handling** (the UI just says "wait 20–30 seconds"), no caching (listed as a mitigation but not built), and no prompt or model version is recorded with results.
- **S2: No evaluation set.** Prompt changes can't be measured, so extraction quality is unknown.
- **S3:** `api/public/index.html` is the Wrangler "Hello World" (it fetches a non-existent `/message`). The `ASSETS` binding and the generic worker name `"api"` are template leftovers.

### E. PDF ingestion (`projection/src/utils/pdfExtractor.ts`)

- **S1: The extractor destroys line structure, which breaks the extracted code.** It joins every text item with `' '` and collapses all whitespace (`:40`), so a multi-line Arduino sketch becomes one line. After that, the first `// comment` comments out **the rest of the program**, while the prompt insists on "preserve the exact code". Tables and pin lists turn to mush the same way.
- **S2: The worker loads from the unpkg CDN** with a hard-coded version string, while `package.json` says `^5.5.207`. The next minor bump causes an API/worker version-mismatch crash. It's also a third-party runtime dependency.
- **S2: The whole PDF goes into one prompt**, with no page or section selection. Real lab manuals contain several experiments, and the model has to guess which one you mean.

### F. Rendering (`projection/src/wokwi/`)

- **S1: XSS.** Component ids from the LLM (and therefore from the uploaded PDF, which is untrusted) are interpolated into `innerHTML` (`CircuitBoard.tsx:324-342`). LLM-supplied `attrs` keys go to `setAttribute(key, …)` (`:359`), so an `attrs: {"onclick": "…"}` executes. Ids also flow unescaped into `querySelector` (`:123`).
- **S2:** Five of the 26 extractable types have **no element** (`capacitor`, `battery`, `wire`, `pir-sensor`, whose real tag is `pir-motion-sensor`, and `hall-effect-sensor`, which is mapped to an analog joystick). `battery` maps to a non-existent `wokwi-vcc` and falls back to a hand-drawn SVG labelled "5V" whatever the voltage.
- **S2:** Grid layout ignores part sizes (a 200×150 grid versus a roughly 274×212 Uno), so parts overlap. Connections to missing components are still emitted. Wires are straight lines.
- **S2:** LEDs are forced `value = true` with brightness 1.0, a visual claim that has nothing behind it.
- **S3:** Refs are read during render (lint errors). React's passive wheel listener makes `preventDefault` a no-op, so the page scrolls while you zoom. Wire recompute relies on a `setTimeout(100)` race.

### G. State and app (`projection/src/store`, `App.tsx`)

- **S2: Two sources of truth.** `App` holds `wokwiDiagram`, `extractedCode` and `status` locally, while the store holds `netlist`, `arduinoCode` and `governanceResult`. The ExtractionResult is cast `as any` into the store (`App.tsx:61`).
- **S2: Race condition.** `setNetlist` triggers an async `solve()` with no sequencing, so a slow earlier solve can overwrite a newer result. WASM `init` runs on every solve.
- **S3: Unused code.** `uiStore` is entirely unused. `solution`, `failures`, `isLoading` and the store's `error` are never rendered, so WASM load failures are invisible. `SAMPLE_CIRCUITS` isn't reachable from the UI and is itself invalid for the kernel (type `'arduino'`, no power source). A comment claims "previous samples kept", but they were deleted.
- **S3:** About 180 lines of inline styles in `App.tsx`. `index.css` is the Vite template and fights with `App.css`.

### H. Build, tooling and repo hygiene

- **S1:** A fresh clone can't `build` or `typecheck`, because nothing runs `wasm-pack` and the prerequisites (`wasm32-unknown-unknown`, `wasm-pack`) aren't documented.
- **S2:** No CI, no workspace (three independent `package-lock.json`s, and `install-all` is broken), and no pinned Rust toolchain.
- **S3:** A 426 KB `worker-configuration.d.ts` is committed but doesn't type `GEMINI_API_KEY`. `.gitignore` lists a `dev.bat` that the docs tell you to use. There's no LICENSE.

### Worth keeping

- The MNA stamps (`stamp_resistor`, `stamp_voltage_source`, `stamp_current_source`) are textbook-correct and well-tested.
- Validating LLM output with a schema and retrying is the right instinct.
- Proxying the key server-side is the right instinct.
- The prompt's anti-hallucination rules are good.
- Switching from the Wokwi iframe to native `@wokwi/elements` was a good call.
- The Rust test discipline (hand-calculated baselines) is a good habit to extend.

---

## 4. Target architecture

### 4.1 Principles

1. **One contract.** Part knowledge lives in exactly one place (the **Part Catalog**), and circuit structure has exactly one canonical shape (the **Circuit IR**). Everything else is generated from, or validated against, those two.
2. **Topology, not spelling.** No physics decision may depend on what a net is *called*.
3. **Analyze, don't gate.** The kernel never blocks rendering. It annotates. Findings are structured (code, severity, component and net ids) and drive the UI highlights.
4. **Structural errors go back to the AI. Physics findings go to the student.**
5. **Honest UI.** Nothing is drawn as "on", "running" or "valid" unless something computed it.
6. **Every layer is testable in isolation**, with fixtures shared across layers.

### 4.2 Pipeline

```text
PDF / pasted text
   │  (layout-preserving extraction, page/section pick)            projection/ingest
   ▼
POST /api/extract { text, pages? }                                   api/ (Worker)
   │  server-owned prompt vN · Gemini structured output (JSON schema from Zod)
   │  → ExtractionIR (lenient: "10k", aliases, evidence quotes)
   │  → normalize() ── structural errors? ──► repair prompt (≤2) ──┐
   │          ▲──────────────────────────────────────────────────────┘
   ▼
CircuitIR (canonical: catalog part ids, canonical pins, SI numbers, nets)   packages/contract
   ├──► kernel.analyze(CircuitIR) → AnalysisReport (WASM)              kernel/ + bridge/
   │       per-net voltages, per-part V/I/P, diagnostics[], coverage
   └──► layout(CircuitIR) → positions → <CircuitBoard> (@wokwi/elements)  projection/render
                           LED lit/brightness/blown ← AnalysisReport
```

`normalize()` lives in a shared TS package, so the **Worker** runs it inside the repair
loop and the **browser** runs it again on load. Same code, same result.

### 4.3 Part Catalog (single source of truth)

A language-neutral data file at `packages/contract/catalog/parts.json`. Rust loads it with
`include_str!` plus serde. TS imports it directly. The prompt's "allowed types" section,
the Zod enum, the Wokwi tag map and the pin alias tables are all **generated** from it.

```jsonc
{
  "id": "led",
  "label": "LED",
  "wokwi": { "tag": "wokwi-led", "attrs": ["color"] },          // attr allowlist
  "pins": [
    { "name": "A", "aliases": ["anode", "+", "a"], "role": "anode" },
    { "name": "C", "aliases": ["cathode", "k", "-", "c"], "role": "cathode" }
  ],
  "model": { "kind": "diode", "params": { "is": 1e-18, "n": 2.0, "rs": 5 } },
  "ratings": { "if_max": 0.02, "vr_max": 5.0 },
  "params": { "color": { "enum": ["red","green","blue","yellow","white"], "default": "red" } },
  "source": "typical 5 mm LED datasheet"                          // every number cites a source
}
```

`model.kind` is one of: `resistor | potentiometer | vsource | diode | switch | open | mcu | unmodeled`.
Parts like `lcd1602` and `dht22` are `unmodeled`. They render, and the report lists them
under coverage instead of pretending they were analyzed.

### 4.4 Circuit IR (canonical)

```jsonc
{
  "version": 1,
  "parts": [
    { "id": "R1",  "type": "resistor", "params": { "resistance": 220 } },
    { "id": "D1",  "type": "led",      "params": { "color": "red" } },
    { "id": "UNO", "type": "arduino-uno" }
  ],
  "nets": [
    { "name": "d13",    "pins": ["UNO.13", "R1.1"] },
    { "name": "led_a",  "pins": ["R1.2", "D1.A"] },
    { "name": "gnd",    "pins": ["D1.C", "UNO.GND.1"], "ground": true }
  ],
  "code": { "language": "cpp", "source": "void setup(){…}" },
  "meta": { "summary": "…", "prompt_version": "extract-v3", "model": "…" }
}
```

- **Nets are the only connectivity.** There are no separate `connections`. Drawn wires are *derived* (a deterministic chain per net).
- Ids match `^[A-Za-z][A-Za-z0-9_]{0,31}$`. Pins are canonical catalog names, and aliases are resolved in `normalize()`.
- Ground is explicit (`ground: true`), or it's inferred by a documented rule: the net holding a `GND*` MCU pin, else the negative terminal of the only source. Otherwise there's no ground, and that's a diagnostic.
- **Source of truth for types:** Rust structs with `serde` + `schemars` + `tsify`/`ts-rs`. That generates `contract.schema.json` and `contract.d.ts`, and CI fails if the generated files are stale. The LLM-facing `ExtractionIR` is Zod-only, because only TS touches it, and `z.toJSONSchema()` feeds Gemini's response schema.

### 4.5 Kernel redesign

Stages, each a pure function with its own tests:

1. **Build graph.** Parse the CircuitIR and resolve pins against the catalog. Union-find merges nets joined by `wire`/closed `switch` (zero-ohm), so there are no 1e9 S stamps. Duplicate ids, unknown pins and pins on two nets are errors.
2. **Topology checks.** These run on the graph, never on names:
   - Voltage-source loop: a source's terminals are in the same merged net (a **short**), or parallel sources have different EMF.
   - Islands: connected components (over modeled parts) with no path to ground. They're excluded from the solve and reported with part ids.
   - Dangling pins: a net with one pin gets a warning, not an error, since open leads and probe points are normal.
3. **Assemble and solve.**
   - Dense MNA stays (it's fine at ≤100 nodes), with deterministic node order (`BTreeMap`) and `gmin = 1e-12 S` per node.
   - Diodes and LEDs use **Newton-Raphson** with a Shockley model, series `rs`, `pnjlim` junction-voltage limiting, and convergence at `reltol 1e-3`, `vntol 1e-6 V`, `abstol 1e-12 A`, with at most 100 iterations. Source-stepping is the fallback.
   - Residual check `‖Ax−z‖/‖z‖ < 1e-9` on the final iterate.
   - Capacitors are open (DC). The potentiometer is two resistors with `wiper` (default 0.5). A switch or pushbutton is open or closed by a `pressed` param (default open).
4. **MCU pins (worst-case static model).** `5V`/`3.3V`/`VIN` are sources, `GND.*` are ground. Any digital or analog pin that drives a load is modeled as a HIGH output (a 5 V Thevenin source with about 25 Ω; the value is sourced from the ATmega328P datasheet's I/O characteristics). This is labelled **"worst case: used pins driven HIGH"** in the report. It gives a real answer to the most common Arduino-lab mistake (an LED on a pin with no resistor) without emulating code.
5. **Rating checks.** Per part, V/I/P are compared against the catalog ratings, and against user overrides when present:
   `S_LED_OVERCURRENT`, `S_LED_REVERSE_V`, `S_RESISTOR_OVERPOWER`, `S_SHORT_CIRCUIT`, `S_PIN_OVERCURRENT` (20 mA recommended, 40 mA absolute max), `S_NO_CURRENT_LIMIT`.
6. **Report:**

```rust
pub struct AnalysisReport {
    pub status: Status,                 // Solved | PartiallySolved | NotSolvable
    pub nets: BTreeMap<NetName, f64>,   // volts, keyed by the caller's names
    pub parts: BTreeMap<PartId, PartReading>,   // v, i, p, state (On/Off/Blown…)
    pub diagnostics: Vec<Diagnostic>,   // { code, severity, message, parts, nets }
    pub coverage: Coverage,             // modeled vs unmodeled part ids
    pub assumptions: Vec<String>,       // "Pins 13 driven HIGH", "SW1 open"
}
```

Error types use `thiserror`. Internal structs are private, with constructors enforcing the
invariants (a real typestate). The magic numbers move into the catalog, each with a cited source.

**Verification:**

- Hand-calculated unit tests (keep the existing ones).
- **Golden fixtures against ngspice.** About 15 reference circuits (LED+R forward and reverse, divider, bridge, pot, diode clipper, MCU pin cases) with `.op` results committed as JSON. The kernel must match within 0.5 %.
- **`proptest`:** random resistor/source networks must satisfy KCL at every node (residual < 1e-9) and be invariant under node renaming and part reordering. That last property would have caught probes #5 and #7.
- Every probe from §2 becomes a regression test.

### 4.6 Bridge

One typed export: `analyze(ir: CircuitIR) -> AnalysisReport`, via `serde-wasm-bindgen` + `tsify`, so there's no JSON string round-trip and no hand-written TS types. Add `kernel_version()` for cache keys. Remove `add` and `greet`. `wasm-opt -Oz` goes in the release profile, and CI tracks size.

### 4.7 Extraction service (Worker)

`POST /api/extract`:

- **Input:** `{ text: string (≤ 60k chars), hint?: string }`. There are no `prompt` or `model` fields from the client.
- **Server:** builds the prompt from versioned templates plus the catalog, and calls Gemini with a JSON response MIME type and a response schema generated from Zod. The model id comes from an env var with a server allowlist. The key goes in the `x-goog-api-key` header.
- **Output items carry an `evidence` field**, a short quote from the source text that justifies each part and net. The UI shows it on hover. It's cheap and catches hallucination quickly. There's also an `uncertain: string[]` list for things the model had to assume.
- **Repair loop, at most 2 rounds, structural only:** schema errors, unknown part or pin, dangling references, duplicate ids, a pin on two nets. Physics findings are **never** sent back.
- **Caching:** KV keyed on `sha256(text ‖ prompt_version ‖ model ‖ catalog_hash)`.
- **429 handling:** honour `Retry-After` with one retry and jitter, then return a typed `rate_limited` error.
- **Abuse controls:** a Cloudflare rate-limiting binding (per IP), a body size cap, CORS origins from env, generic error bodies (details only in logs), and logs that never contain the text.
- **Response:** `{ ir: CircuitIR, extraction_meta: { prompt_version, model, repairs, cached } }`.

**Evaluation harness** (`eval/`):

- 15–25 real or representative handout excerpts, each with a hand-labelled `expected.ir.json`. Cover: blink, LED+R, divider, button with pull-down, pot→A0, series-parallel lab, diode, servo, ultrasonic, a **multi-experiment manual**, **text containing an injection attempt**, and **text with no circuit**.
- Scorer: part P/R (type + value within 5 %), **pin-pair connectivity F1** (for every pin pair, same net or not; this is isomorphism-robust), code match after whitespace normalization, and a hallucination count (parts with no evidence match).
- `npm run eval` runs live (manually or nightly). CI runs the pipeline against **recorded** model responses, so it stays deterministic and free.
- A prompt or model change merges only if eval scores don't regress.

### 4.8 Frontend

- **One Zustand store:** `source` (text and page range) → `ir` → derived `analysis` and `layout`. Local state is removed from `App`. Kernel calls are sequenced with a monotonically increasing request id, and stale results are dropped. WASM is initialised once (a module-level promise).
- **Ingest:** use pdf.js `hasEOL` plus y-position grouping to rebuild lines and preserve indentation. Only runs of spaces *within* a line are collapsed. The worker is bundled via `pdfjs-dist/build/pdf.worker.min.mjs?url`, with no CDN. A page-range picker is shown before extraction.
- **Render:**
  - Elements are built with `document.createElement` only, never `innerHTML`.
  - attrs are filtered through the catalog allowlist, and selectors use `CSS.escape`.
  - A size-aware grid uses `getBoundingClientRect` after upgrade, with MCUs placed first.
  - Wires come from the derived per-net chain.
  - Parts without an element get an explicit labelled placeholder, one per catalog `id`.
  - **LED state comes from `analysis`**: lit if I > 1 mA, brightness ∝ I/If_max, and a blown marker on `S_LED_OVERCURRENT`.
  - Non-passive wheel listener. Drag and pan state lives in `useState`/`useReducer`, not refs read during render.
- **Findings panel** (replaces the pass/fail box): diagnostics grouped by severity. Clicking one highlights its parts and nets. It includes a coverage line ("Analyzed 4 of 6 parts; LCD1 and DHT1 are not simulated") and the assumptions list.
- **Readings table:** net voltages and per-part V/I/P from the solve. This is the data the solver already computes and currently throws away, and it's the "expected readings" half of the product.
- **Code panel:** labelled **"Code from handout (not executed)"**.
- Styles move from inline objects to CSS modules. `index.css` template leftovers are deleted.

---

## 5. Execution plan

Each phase leaves `main` green (CI passing) and is independently reviewable. Sizes are
relative (S ≈ ½ day, M ≈ 1–2 days, L ≈ 3–5 days).

### Phase 0: Truth and hygiene (S)

- [ ] Replace `MASTER_PLAN.md` with `ROADMAP.md`, with statuses that match reality (§1). Move the historical plan to `docs/history/`.
- [ ] Write a root `README.md` (honest one-paragraph pitch, prerequisites, quickstart) and `ARCHITECTURE.md` (a condensed §4).
- [ ] Delete the cruft: `api/public/` plus the `assets` block, bridge `add`/`greet`, the unused `uiStore`, invalid `SAMPLE_CIRCUITS`, the template READMEs, `index.css` template rules, the `dev.bat` gitignore line, and the "3D projection" and "York" comments.
- [ ] Add `rust-toolchain.toml` (stable + `wasm32-unknown-unknown`). Root npm **workspaces** (`api`, `projection`, `packages/*`), which removes `install-all`. Root scripts: `wasm`, `dev` (runs `wasm` first), `build`, `test`, `lint`, `typecheck`.
- [ ] Add CI (`.github/workflows/ci.yml`): `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `wasm-pack build`, `npm ci`, `tsc`, `eslint`, `vitest`.
- **Accept:** a fresh clone runs `npm i && npm run build && npm test` and it passes. Lint is clean.

### Phase 1: The contract (M)

- [ ] `packages/contract`: `catalog/parts.json` (every part the prompt allows today, with Wokwi tag, pins and aliases, model kind, ratings and a source for each number), plus the Rust `CircuitIR` crate module with `schemars`/`tsify` codegen and a CI staleness check.
- [ ] Zod `ExtractionIR` (nets-first, `evidence`, lenient values) and `normalize(ExtractionIR) → { ir, structuralErrors[] }`, covering alias resolution, SI parsing (`"4.7k"`, `"10 µF"`, `"220Ω"`, `"1M"`) and id/pin/net checks.
- [ ] Generate from the catalog: the prompt part list, the Zod enum, the Wokwi tag map and the attr allowlist. Delete `TYPE_MAP`, `TAG_MAP`, `mapPin` and `types.ts`.
- [ ] Resolve the catalog gaps: `pir-sensor` → `pir-motion-sensor`; drop `hall-effect-sensor` (no element, no model) or mark it `unmodeled` with a placeholder; give `battery`/`capacitor`/`wire` explicit placeholders.
- **Accept:** a Vitest table of 40+ `normalize()` cases, and the SI parser fuzzed with `fast-check`. The prompt text is a snapshot test.

### Phase 2: Kernel rebuild (L)

- [ ] Stages 1–6 from §4.5, in that order, each with unit tests. Keep the existing stamp functions and baseline tests.
- [ ] Newton-Raphson diode/LED model plus limiting and source-stepping.
- [ ] MCU worst-case pin model, clearly labelled as an assumption.
- [ ] `AnalysisReport` with coded diagnostics, `thiserror`, private internals and deterministic ordering.
- [ ] Every §2 probe becomes a regression test, plus the ngspice golden fixtures and `proptest` invariants (KCL, renaming, reordering).
- **Accept:** all 14 probes produce the "Should be" column. Golden fixtures match within 0.5 %. Proptest runs 10k cases green.

### Phase 3: Bridge (S)

- [ ] `analyze(ir) -> AnalysisReport` via `serde-wasm-bindgen` + `tsify`, plus `kernel_version()`. Size budget in CI.
- **Accept:** a Vitest test (Node, using the WASM build) runs one fixture end-to-end and asserts typed fields.

### Phase 4: Extraction service (M)

- [ ] `POST /api/extract` as in §4.7. Remove `/api/gemini`. The prompt is versioned server-side, with structured output, the structural-only repair loop, the KV cache and `Retry-After` handling.
- [ ] Hono tests via `app.request()` with a mocked `fetch`: happy path, repair path, 429, oversize body, a disallowed model field (ignored), and CORS.
- [ ] Typed env (`GEMINI_API_KEY`, `GEMINI_MODEL`, `ALLOWED_ORIGINS`, KV and rate-limit bindings) via `wrangler types`.
- **Accept:** API tests are green. `curl` with a foreign `Origin` and a big body is rejected. The key never appears in a URL.

### Phase 5: Evaluation harness (M, can run parallel to Phase 4)

- [ ] `eval/fixtures/*`, scorer, recorded-response replay in CI, and `npm run eval` live mode that writes `eval/report.md`.
- **Accept:** a baseline score is recorded for the current prompt. CI fails on a scorer regression in replay mode.

### Phase 6: Ingest and frontend integration (L)

- [ ] Line-preserving PDF extraction, a bundled worker and a page picker. Test on three fixture PDFs, one of which contains a multi-line sketch with `//` comments that has to survive intact.
- [ ] Single store, sequenced analysis and a WASM singleton.
- [ ] `CircuitBoard` rewrite: no `innerHTML`, catalog attr allowlist, size-aware layout, net-derived wires, analysis-driven LEDs and fixed lint issues.
- [ ] Findings panel, readings table, and a code panel labelled "not executed".
- [ ] Component tests (Vitest + Testing Library) for the store and panels. One Playwright smoke test: paste fixture text → mocked API → board renders → findings show `S_LED_OVERCURRENT` for the no-resistor fixture.
- **Accept:** a malicious id (`<img src=x onerror=alert(1)>`) and a malicious `attrs.onclick` both render inert (Playwright test). Playwright is green.

### Phase 7: Hardening and deploy readiness (S)

- [ ] `_headers` on Pages with a strict CSP (`script-src 'self' 'wasm-unsafe-eval'`, no inline scripts), plus `connect-src` limited to the API origin.
- [ ] A privacy note in the UI ("handout text is sent to Google Gemini for extraction").
- [ ] Deploy docs (secrets, KV namespace, rate-limit binding, CORS origins).
- **Accept:** a security checklist in the PR is fully ticked.

**Critical path:** 0 → 1 → (2 ∥ 4 ∥ 5) → 3 → 6 → 7.

---

## 6. Decisions I need from you

1. **Code execution (emulation).** The docs promise that Arduino code runs, and nothing runs it. Options:
   - **(A, recommended)** Be honest now: "visualize + analyze". Add an **"Open in Wokwi"** export (a `diagram.json` + `sketch.ino` bundle the student pastes into wokwi.com). That's small, and it honours the promise through Wokwi itself.
   - **(B)** In-browser AVR emulation with `avr8js`. This needs a compile service (`arduino-cli` in a container; it can't run on Workers). It's a large new surface, so it's out of scope unless you want it.
2. **Where the IR types live.** I recommend Rust as the source (codegen to TS and JSON Schema), with the LLM-facing schema in Zod. The alternative is Zod-first with Rust mirroring it by hand. That's simpler tooling, but the drift risk returns.
3. **Worst-case MCU pin model** (§4.5 step 4). It's the most valuable safety check for Arduino labs. It's an explicit assumption and is labelled as one. OK to include?
4. **Section selection.** The plan picks a page range. An LLM "which experiment?" pre-pass is the smarter version, but it's one extra call. Page range first?
5. **License.** None today. Pick one (MIT/Apache-2.0) before any public deploy.

## 7. Deliberately out of scope

The circuit editor (5.1.1), vision extraction for diagrams (3.2.4), transient/AC analysis,
PWA, save/load and share links, and the TA feedback system. The contract and single-store
design make each of these straightforward later, but none is needed to make what exists correct.

---

## Appendix: probe harness

The probe harness is a throwaway Cargo binary that depends on `labwise-kernel` by path.
It calls `GovernanceManager::validate_json` and then `solve_circuit` on each case in §2.
It will be committed as `kernel/tests/regressions.rs` in Phase 2, with assertions flipped to the
"Should be" column.
