# assess — Decision Framing

## One-line promise

**Deterministic decision classification over a complete spine evidence bundle — PROCEED, PROCEED_WITH_RISK, ESCALATE, or BLOCK — using versioned, declared policy rules evaluated against the whole bundle.**

---

## Problem

After `shape`, `rvl`, `verify`, and `benchmark` produce their reports, someone (or something) needs to decide: do we continue, escalate, or stop?

Today this is ad-hoc agent logic — every pipeline invents its own "is this good enough?" threshold. `assess` replaces that with declared, versioned, deterministic policy. Humans review policies, not ad-hoc decisions.

### Core principle

Truth remains binary. Decisions are graded. `assess` does not change what is true — it classifies results into actionability bands using declared, deterministic rules.

---

## Non-negotiables (engineering contract)

These are not aspirations. They are the contract. If any are violated, assess is not assess yet.

1. **Deterministic** — Same artifacts + same policy = same decision, byte-for-byte identical output. No randomness, no ambient state, no timestamps in the decision.
2. **No expression evaluation** — v0 match surface is exact equality only. No numeric comparisons, no dot-path traversal, no CEL, no evalexpr. If a future version needs thresholds, it gets a new schema version.
3. **No HashMap in JSON output** — All output uses `BTreeMap` or insertion-ordered serialization. Golden rule `no-hashmap-in-output.yml` enforced.
4. **Policy is content-hashed** — The `sha256` field in the output must equal the SHA-256 of the policy file's raw bytes. If they diverge, the output is untrustworthy.
5. **Every input accounted for** — `assess` never silently drops or ignores an artifact. Every provided artifact appears in `epistemic_basis`. Extra artifacts (beyond `requires`) are recorded but do not affect the decision.
6. **One decision per invocation** — `assess` produces exactly one JSON object to stdout. No batching, no streaming, no multi-document output.
7. **`#![forbid(unsafe_code)]`** — No unsafe blocks anywhere in the crate.

---

## Scope boundary

### In scope (v0)

- Load and validate YAML policy files (`policy.v0` schema)
- Load and parse JSON spine artifacts
- Resolve canonical tool identity from top-level `tool` when present, otherwise derive it from artifact `version`
- Validate `requires` completeness and tool uniqueness
- Match the artifact bundle against ordered `when` clauses (exact equality)
- Emit a single `assess.v0` JSON decision to stdout
- Human-readable output mode (default)
- Witness ledger integration (ambient recording, queryable subcommand)
- operator.json self-description
- spine-rules golden rule enforcement

### Excluded (not assess's job)

- Producing facts (that's `rvl`, `verify`, `shape`, `benchmark`)
- Diffing datasets (that's `compare`)
- Scoring extractions (that's `benchmark`)
- Factory conflict resolution (separate tool, see PLAN_FACTORY.md)
- Tournament ranking (consumers use raw `benchmark.summary.accuracy` after assess gates)
- Arbitrary JSON ingestion (input domain is spine reports only)
- Probabilistic or ML-based classification

### Deferred (future versions)

- **Expression evaluation (v1):** Numeric thresholds via CEL (`checks.schema_overlap.overlap_ratio >= 0.45`). Deferred because the narrowed v0 match surface makes expressions unnecessary — tools emit discrete `policy_signals` bands instead.
- **Policy backtesting / replay (v1+):** Re-running a policy against historical evidence packs. Deferred because (a) `pack` has no corpus yet, and (b) the discrete match surface makes policy diffs trivially predictable by reading the YAML.
- **`--validate` subcommand (v1):** Schema-validate a policy file without running an assessment. Low-priority convenience; `E_BAD_POLICY` on real invocations serves the same purpose.
- **Policy inheritance / composition:** Policies that extend or override other policies. Adds complexity without clear need in v0.

### v0 bar

`assess` is done when: a clean `shape+rvl+verify` bundle assessed against the loan tape policy produces PROCEED, a bundle with `E_DIFFUSE` produces ESCALATE, the witness ledger records both, and the output passes `pack seal` as a valid member artifact.

---

## Non-goals

`assess` is NOT:
- A truth tool (that's `rvl`, `verify`, `shape`)
- A diff tool (that's `compare`)
- A scoring tool (that's `benchmark`)
- A general JSON rules engine
- The factory's conflict resolver
- Probabilistic or ML-based

It does not produce facts. It classifies facts into decisions.

---

# Part I — Design Specification

---

## CLI

```
assess <ARTIFACT>... --policy <POLICY> [OPTIONS]

Arguments:
  <ARTIFACT>...          Spine reports to assess (shape, rvl, verify, benchmark, fingerprint results)

Options:
  --policy <PATH|ID>     Decision policy file or ID
  --policy-id <ID>       Policy ID (resolved from search path)
  --json                 JSON output (default: human-readable)
  --no-witness           Suppress witness ledger recording
```

One or more artifacts and a policy are required at the CLI level, but the policy's `requires` list must be fully satisfied or `assess` refuses with `E_INCOMPLETE_BASIS`.

`--policy` and `--policy-id` are mutually exclusive; providing both is a refusal (`E_AMBIGUOUS_POLICY`).

### Policy resolution order

1. `ASSESS_POLICY_PATH` env var (colon-separated directories)
2. Built-in policies bundled with the binary
3. `~/.epistemic/policies/` if present

### Exit codes

`0` PROCEED | `1` PROCEED_WITH_RISK or ESCALATE | `2` BLOCK or refusal

Exit code semantics: `0` means the pipeline can continue without annotation. `1` means the pipeline can continue but requires attention (risk annotation or human review). `2` means stop.

---

## Decision bands

| Band | Meaning | Agent behavior | Exit code |
|------|---------|----------------|-----------|
| `PROCEED` | Epistemically clean | Continue pipeline | `0` |
| `PROCEED_WITH_RISK` | Known, bounded deficiencies | Continue + annotate risk factors | `1` |
| `ESCALATE` | Judgment required | Route to human | `1` |
| `BLOCK` | Epistemically unsound | Stop pipeline | `2` |

No soft synonyms. No percentages without definition.

---

## Output (JSON)

```json
{
  "version": "assess.v0",
  "decision_band": "ESCALATE",
  "policy": {
    "id": "loan_tape.monthly.v1",
    "version": 1,
    "sha256": "sha256:a1b2c3..."
  },
  "matched_rule": "diffuse_requires_review",
  "required_tools": ["shape", "rvl", "verify"],
  "observed_tools": ["shape", "rvl", "verify"],
  "risk_factors": [
    {
      "code": "DIFFUSE_CHANGE",
      "source_tool": "rvl",
      "detail": "rvl refused with E_DIFFUSE; human review required"
    }
  ],
  "epistemic_basis": [
    {
      "artifact": "shape.report.json",
      "tool": "shape",
      "version": "shape.v0",
      "outcome": "COMPATIBLE",
      "policy_signals": { "compatibility_band": "FULL" }
    },
    {
      "artifact": "rvl.report.json",
      "tool": "rvl",
      "version": "rvl.v0",
      "outcome": null,
      "policy_signals": {},
      "refusal": { "code": "E_DIFFUSE" }
    },
    {
      "artifact": "verify.report.json",
      "tool": "verify",
      "version": "verify.report.v1",
      "outcome": "PASS",
      "policy_signals": {}
    }
  ],
  "refusal": null
}
```

---

## Policy file schema (`policy.v0`)

A policy is not a model. It is a deterministic rule set over a declared basis, versioned and auditable.

```yaml
schema_version: 1
policy_id: loan_tape.monthly.v1
policy_version: 1
description: "Monthly loan tape reconciliation policy"
requires:
  - shape
  - rvl
  - verify

rules:
  - name: clean_reconciliation
    when:
      shape:
        outcome: COMPATIBLE
      rvl:
        outcome_in: [REAL_CHANGE, NO_REAL_CHANGE]
      verify:
        outcome: PASS
    then:
      decision_band: PROCEED

  - name: partial_overlap_acceptable
    when:
      shape:
        outcome: INCOMPATIBLE
        signals:
          compatibility_band: PARTIAL
      rvl:
        outcome_in: [REAL_CHANGE, NO_REAL_CHANGE]
      verify:
        outcome: PASS
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: PARTIAL_SCHEMA_OVERLAP

  - name: diffuse_requires_review
    when:
      shape:
        outcome: COMPATIBLE
      rvl:
        refusal: E_DIFFUSE
      verify:
        outcome: PASS
    then:
      decision_band: ESCALATE
      risk_code: DIFFUSE_CHANGE

  - name: tolerable_missingness
    when:
      shape:
        outcome: COMPATIBLE
      rvl:
        refusal: E_MISSINGNESS
        signals:
          missingness_band: TOLERABLE
      verify:
        outcome: PASS
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: MISSINGNESS_TOLERATED

  - name: verify_fail_requires_review
    when:
      verify:
        outcome: FAIL
    then:
      decision_band: ESCALATE
      risk_code: VERIFY_FAIL

  - name: default_block
    default: true
    then:
      decision_band: BLOCK
      risk_code: UNHANDLED_CONDITION
```

---

## Policy signals

To keep `assess` narrow without losing useful nuance, tools that currently expose only raw metrics add **discrete, tool-owned `policy_signals`**. `assess` may match these exact values, but it does not compare raw numbers or traverse arbitrary JSON.

- `shape.policy_signals.compatibility_band`: `FULL | PARTIAL | BROKEN`
- `rvl.policy_signals.missingness_band`: `TOLERABLE | SEVERE` (only when `refusal.code = E_MISSINGNESS`)
- `verify.policy_signals.severity_band`: `CLEAN | WARN_ONLY | ERROR_PRESENT`
- `benchmark.policy_signals.quality_band`: `HIGH | ACCEPTABLE | LOW`

This keeps the line clean:
- the producing tool owns the raw metrics and how they collapse into a stable band
- `assess` only combines discrete states across the full bundle

### Policy rules using signals

```yaml
  - name: partial_overlap_acceptable
    when:
      shape:
        outcome: INCOMPATIBLE
        signals:
          compatibility_band: PARTIAL
      rvl:
        outcome_in: [REAL_CHANGE, NO_REAL_CHANGE]
      verify:
        outcome: PASS
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: PARTIAL_SCHEMA_OVERLAP

  - name: cross_artifact_warn_only
    when:
      verify:
        outcome: FAIL
        signals:
          severity_band: WARN_ONLY
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: CROSS_ARTIFACT_WARNINGS

  - name: benchmark_low_accuracy
    when:
      benchmark:
        signals:
          quality_band: LOW
    then:
      decision_band: BLOCK
      risk_code: EXTRACTION_QUALITY_UNACCEPTABLE
```

---

## Tool identity resolution

`assess` needs one canonical tool name per artifact for:

- `requires`
- duplicate detection
- rule matching
- `observed_tools`
- `epistemic_basis[*].tool`
- `risk_factors[*].source_tool`

V0 resolution rule:

1. if top-level `tool` exists, use it
2. otherwise derive the tool name by stripping the `.v<N>` suffix from `version`

Examples:

- `{ "tool": "verify", "version": "verify.report.v1" }` → `verify`
- `{ "version": "benchmark.v0" }` → `benchmark`
- `{ "version": "shape.v0" }` → `shape`

This keeps `assess` compatible with both:

- newer explicit-tool reports like `verify.report.v1`
- legacy reports that only expose `version`

`version` remains required even when `tool` is present. The explicit `tool`
field is authoritative for tool identity; `version` continues to identify the
artifact contract.

## Rule evaluation

- At most one artifact per canonical tool identity is allowed; duplicates are a refusal (`E_DUPLICATE_TOOL`)
- The policy's `requires` list must be a subset of observed canonical tool identities; otherwise `assess` refuses with `E_INCOMPLETE_BASIS`
- Rules are evaluated in order against the **whole bundle**; the first rule whose `when` clause matches the bundle wins
- A `when` clause may match only:
  - `outcome`
  - `outcome_in`
  - `refusal`
  - exact-equality `signals` under `policy_signals`
- `assess v0` does **not** evaluate arbitrary expressions, numeric thresholds, or dot-path comparisons
- `default: true` marks the fallback rule (strongly recommended and must be last when present)
- All rules must specify `decision_band`; `risk_code` is required for non-PROCEED bands
- If no rule matches (and no default exists), `assess` refuses with `E_MISSING_RULE`
- Tournament ranking is explicitly out of scope; `assess` only gates

---

## Extensibility

`assess` does not hardcode which tools it understands. Policy rules match on tool keys in the `when` clause. Any structured report with a `version` and `outcome` and/or `refusal` field can be assessed. If the report exposes top-level `tool`, that value is authoritative; otherwise `assess` falls back to version-derived tool identity. For example, a factory conflict resolution engine might emit:

```json
{
  "version": "conflict.v0",
  "outcome": "DISPUTED",
  "policy_signals": { "tolerance_exceeded": true }
}
```

A policy can reference this:

```yaml
  - name: conflict_unresolved
    when:
      conflict:
        outcome: DISPUTED
        signals:
          tolerance_exceeded: true
    then:
      decision_band: ESCALATE
      risk_code: CONFLICT_UNRESOLVED
```

---

## What this changes about refusals

Refusals do not change. What changes: refusals become inputs to `assess`, not terminal states.

- `rvl → REFUSAL (E_DIFFUSE)` — epistemic truth unchanged
- `assess` deterministically maps: `E_DIFFUSE → ESCALATE`
- `E_MISSINGNESS` with `policy_signals.missingness_band = TOLERABLE` → `PROCEED_WITH_RISK`

You preserve honesty and momentum.

---

## Usage examples

```bash
# Assess a reconciliation against a policy
shape nov.csv dec.csv --json > shape.json
rvl nov.csv dec.csv --json > rvl.json
verify dec.csv --rules rules.json --json > verify.json

assess shape.json rvl.json verify.json \
  --policy loan_tape.monthly.v1 \
  > decision.json

# Minimal: just one report
assess rvl.json --policy default.v0 > decision.json

# assess → pack: decision becomes part of evidence
assess shape.json rvl.json verify.json --policy loan_tape.monthly.v1 > decision.json
pack seal shape.json rvl.json verify.json decision.json nov.lock.json dec.lock.json \
  --note "Nov→Dec recon with decision" --output evidence/2025-12/
```

---

# Part II — Implementation Plan

---

## Module skeleton

```
assess/
├── Cargo.toml
├── operator.json
├── rules/
│   ├── exit-code-range.yml
│   ├── no-hashmap-in-output.yml
│   └── witness-must-append.yml
├── src/
│   ├── main.rs                  — Thin entry point; calls lib::run(), maps exit code
│   ├── lib.rs                   — Module tree, pub fn run(args) -> Result<ExitCode>
│   ├── cli/
│   │   ├── mod.rs               — Argument parsing and command routing
│   │   ├── args.rs              — clap::Parser derive struct (Args)
│   │   └── exit.rs              — ExitCode enum: Proceed(0), Attention(1), Stop(2)
│   ├── policy/
│   │   ├── mod.rs               — Policy loading orchestrator (resolve → read → parse → validate)
│   │   ├── loader.rs            — Resolution order: env var → builtin → user dir
│   │   ├── schema.rs            — Policy struct types (PolicyFile, Rule, WhenClause, ThenClause)
│   │   └── validate.rs          — Schema validation (required fields, rule well-formedness, default-last)
│   ├── bundle/
│   │   ├── mod.rs               — Artifact bundle construction
│   │   ├── artifact.rs          — Single artifact types (tool, version, outcome, refusal, signals)
│   │   └── derive.rs            — Canonical tool identity resolution (explicit `tool` first, version fallback)
│   ├── evaluate/
│   │   ├── mod.rs               — Evaluation orchestrator (check requires → match rules → emit decision)
│   │   └── matcher.rs           — Rule matching: when-clause against bundle (exact equality)
│   ├── output/
│   │   ├── mod.rs               — Output dispatch (--json vs human)
│   │   ├── json.rs              — assess.v0 JSON serialization (BTreeMap, ordered keys)
│   │   └── human.rs             — Human-readable summary (decision band, risk factors, basis)
│   ├── refusal/
│   │   ├── mod.rs               — Refusal routing and envelope construction
│   │   ├── codes.rs             — RefusalCode enum (E_BAD_POLICY, E_DUPLICATE_TOOL, etc.)
│   │   └── payload.rs           — Refusal detail struct (code, trigger, next)
│   └── witness/
│       ├── mod.rs               — Witness ledger management (append, suppress)
│       ├── ledger.rs            — JSONL append to ~/.epistemic/witness.jsonl
│       ├── query.rs             — `assess witness` subcommand (query, last, count)
│       └── record.rs            — Witness entry schema (tool="assess", inputs, decision, duration)
├── tests/
│   ├── golden_rules.rs          — spine-rules framework tests
│   ├── policy_load.rs           — Policy loading and validation
│   ├── bundle_construct.rs      — Artifact parsing and tool derivation
│   ├── evaluate_rules.rs        — Rule matching and decision logic
│   ├── refusal_suite.rs         — All 7 refusal codes exercised
│   ├── output_schema.rs         — JSON output validates against assess.v0 schema
│   ├── witness_suite.rs         — Witness append, suppress, query
│   ├── e2e_pipeline.rs          — Full pipeline: artifacts → assess → pack-compatible output
│   └── determinism.rs           — Same inputs → identical output (byte-level)
└── fixtures/
    ├── policies/
    │   ├── loan_tape_monthly_v1.yaml
    │   └── minimal_default_only.yaml
    ├── artifacts/
    │   ├── shape_compatible.json
    │   ├── shape_incompatible_partial.json
    │   ├── rvl_real_change.json
    │   ├── rvl_no_real_change.json
    │   ├── rvl_refusal_diffuse.json
    │   ├── rvl_refusal_missingness_tolerable.json
    │   ├── verify_pass.json
    │   ├── verify_fail.json
    │   └── benchmark_low.json
    └── golden/
        ├── proceed.json
        ├── proceed_with_risk.json
        ├── escalate.json
        └── block.json
```

**Dependency direction:** `main → lib → cli → {policy, bundle, evaluate, output, refusal, witness}`. `evaluate` depends on `policy` and `bundle`. `output` depends on `evaluate` (for the decision type). No cycles.

---

## Data model invariants

| ID | Invariant | Protects against |
|----|-----------|------------------|
| I01 | Same artifacts + same policy = same decision, byte-for-byte | Non-determinism from HashMap, timestamps, or ambient state |
| I02 | The output `decision_band` equals exactly the `then.decision_band` of the matched rule | Aggregation bugs, implicit band promotion/demotion |
| I03 | If any tool in `requires` is absent from the bundle, assess refuses `E_INCOMPLETE_BASIS` | Partial evidence producing false confidence |
| I04 | At most one artifact per canonical tool identity in the bundle | Ambiguous state from duplicate tool reports |
| I05 | Rules are evaluated in declaration order; first match wins | Order-dependent rule sets producing wrong decisions if evaluation order changes |
| I06 | The default rule (if present) must be last; it always matches when reached | Rules after default being dead code |
| I07 | `policy.sha256` in output = SHA-256 of the policy file's raw bytes | Stale or tampered policy hash making output unverifiable |
| I08 | Every non-PROCEED decision band has a non-empty `risk_code` in the rule | Risk-bearing decisions with no machine-readable reason |
| I09 | All refusals exit 2; BLOCK exits 2; PROCEED exits 0; PROCEED_WITH_RISK and ESCALATE exit 1 | Pipeline automation misinterpreting the decision |
| I10 | Witness record is appended for every invocation unless `--no-witness` is set | Lost audit trail for production decisions |
| I11 | Every provided artifact appears in `epistemic_basis`, even if not in `requires` | Silent artifact dropping |
| I12 | `matched_rule` in output is the `name` field of the rule that won | Output not traceable to the specific policy rule |
| I13 | Canonical tool identity uses top-level `tool` when present, otherwise falls back to version-derived identity | Incompatibility between explicit-tool reports and legacy version-only reports |

---

## Error taxonomy

### External refusal codes (CLI output)

| Code | Trigger | Exit | Emitting module |
|------|---------|------|-----------------|
| `E_BAD_POLICY` | YAML parse failure, missing required fields, rule without `decision_band`, non-PROCEED rule without `risk_code`, default rule not last | 2 | `policy::validate` |
| `E_AMBIGUOUS_POLICY` | Both `--policy` and `--policy-id` provided | 2 | `cli::args` |
| `E_UNKNOWN_POLICY` | Policy ID not found in any resolution path | 2 | `policy::loader` |
| `E_BAD_ARTIFACT` | JSON parse failure, missing `version` field, malformed explicit `tool`, or version field doesn't match `*.v<N>` when fallback derivation is needed | 2 | `bundle::artifact` |
| `E_DUPLICATE_TOOL` | Two or more artifacts resolve to the same canonical tool identity | 2 | `bundle::mod` |
| `E_INCOMPLETE_BASIS` | Policy `requires` lists a tool not present in the bundle | 2 | `evaluate::mod` |
| `E_MISSING_RULE` | No rule matched and no default rule exists | 2 | `evaluate::matcher` |

### Internal error types (Rust)

```rust
// policy/mod.rs
enum PolicyError {
    Io(std::io::Error),           // File read failure
    YamlParse(serde_yaml::Error), // YAML syntax error
    SchemaViolation(String),      // Structural validation failure
    NotFound { id: String },      // Resolution failed
    AmbiguousSelector,            // --policy + --policy-id
}

// bundle/mod.rs
enum BundleError {
    Io(std::io::Error),
    JsonParse { path: PathBuf, source: serde_json::Error },
    NoVersion { path: PathBuf },
    BadTool { path: PathBuf, tool: String },
    BadVersion { path: PathBuf, version: String },
    DuplicateTool { tool: String, paths: [PathBuf; 2] },
}

// evaluate/mod.rs
enum EvalError {
    IncompleteBasis { missing: Vec<String> },
    NoMatchingRule,
}
```

Each internal error maps to exactly one refusal code. `PolicyError::Io` and `PolicyError::YamlParse` both map to `E_BAD_POLICY`.

---

## Contract table

| ID | Scope | Behavioral contract |
|----|-------|---------------------|
| C01 | Exit codes | PROCEED → 0, PROCEED_WITH_RISK → 1, ESCALATE → 1, BLOCK → 2, any refusal → 2 |
| C02 | Refusal envelope | Every refusal produces valid JSON: `{ "version": "assess.v0", "decision_band": null, "refusal": { "code": "E_...", "detail": "...", "next": "..." } }` |
| C03 | Policy loading | Policies resolve in order: `ASSESS_POLICY_PATH` → builtins → `~/.epistemic/policies/`. First match wins. |
| C04 | Policy validation | Invalid policies produce `E_BAD_POLICY` with a diagnostic message identifying the specific violation |
| C05 | Bundle construction | Canonical tool identity uses explicit top-level `tool` when present, otherwise version-derived fallback. Duplicates refused. Every artifact recorded in `epistemic_basis`. |
| C06 | Rule matching | Rules evaluated in declaration order against the whole bundle. First rule whose `when` clause matches wins. Match surface: `outcome`, `outcome_in`, `refusal`, `signals` (exact equality only). |
| C07 | Default rule | `default: true` always matches when reached. Must be last rule. If no rule matches and no default exists, refusal `E_MISSING_RULE`. |
| C08 | Output schema | JSON output conforms to `assess.v0` schema. `matched_rule`, `required_tools`, `observed_tools`, `risk_factors`, `epistemic_basis` all present. |
| C09 | Witness | Every invocation appends a witness record to `~/.epistemic/witness.jsonl` unless `--no-witness`. Record includes: tool name, input paths, policy ID, decision band, duration. |
| C10 | Pipeline compat | `assess` output is a valid member artifact for `pack seal`. The `version` field is `assess.v0`. |
| C11 | Determinism | Identical inputs (same artifact bytes, same policy bytes) produce byte-identical output across runs, platforms, and Rust compiler versions. |
| C12 | Policy hash | `policy.sha256` = SHA-256 of the policy file's raw bytes, hex-encoded with `sha256:` prefix. |

---

## Threat table

| ID | Threat | Mitigation | Refusal code |
|----|--------|------------|--------------|
| T01 | Malformed policy YAML (syntax errors, wrong types, missing fields) | `policy::validate` rejects before evaluation begins | `E_BAD_POLICY` |
| T02 | Duplicate artifacts for the same canonical tool identity | `bundle::mod` checks after tool resolution | `E_DUPLICATE_TOOL` |
| T03 | Incomplete evidence (policy requires tools not in bundle) | `evaluate::mod` checks `requires` before rule matching | `E_INCOMPLETE_BASIS` |
| T04 | Policy with no default rule and unmatched bundle state | `evaluate::matcher` returns `EvalError::NoMatchingRule` | `E_MISSING_RULE` |
| T05 | Both `--policy` and `--policy-id` provided | `cli::args` validates mutual exclusivity at parse time | `E_AMBIGUOUS_POLICY` |
| T06 | Artifact JSON with missing `version`, malformed explicit `tool`, or non-derivable fallback version | `bundle::artifact` validates canonical tool resolution inputs | `E_BAD_ARTIFACT` |
| T07 | Empty artifact list (no positional args) | `clap` enforces `required = true` on positional args | clap error (not a refusal) |
| T08 | Policy containing `condition` fields from pre-narrowing spec | `policy::validate` rejects unknown fields in `when` clauses | `E_BAD_POLICY` |
| T09 | Non-deterministic output from HashMap serialization | `#![forbid]` on HashMap in output types; golden rule enforced | Build-time / test-time |
| T10 | Artifact with `outcome: null` but no `refusal` block | Valid state (tool ran but produced no outcome and no refusal). Recorded in `epistemic_basis` but may not match any rule. | Falls through to default or `E_MISSING_RULE` |

---

## Staged implementation sequence

### D1 — CLI scaffold + exit codes + refusal skeleton

**Build:** `cli/args.rs`, `cli/exit.rs`, `cli/mod.rs`, `refusal/codes.rs`, `refusal/payload.rs`, `refusal/mod.rs`, `main.rs`, `lib.rs`

**Satisfies:** C01, C02

**Gate:** `assess --help` prints usage. `assess` with no args exits 2 with valid refusal JSON. All 7 refusal codes are defined as enum variants.

### D2 — Policy loader + validation

**Build:** `policy/schema.rs`, `policy/loader.rs`, `policy/validate.rs`, `policy/mod.rs`

**Satisfies:** C03, C04, C12, I07, I08

**Gate:** Valid YAML loads into typed `PolicyFile`. Invalid YAML produces `E_BAD_POLICY` with diagnostic. Policy SHA-256 computed and matches raw file hash. Rules without `risk_code` on non-PROCEED bands are rejected. Default rule not-last is rejected.

**Depends on:** D1

### D3 — Bundle construction + tool resolution

**Build:** `bundle/artifact.rs`, `bundle/derive.rs`, `bundle/mod.rs`

**Satisfies:** C05, I03, I04, I11

**Gate:** JSON artifacts parse into typed structs. Canonical tool identity resolves from explicit top-level `tool` when present, otherwise from `version`. Duplicate canonical tool identities produce `E_DUPLICATE_TOOL`. Missing version field or malformed explicit tool produces `E_BAD_ARTIFACT`. All artifacts recorded.

**Depends on:** D1

### D4 — Rule matcher + evaluation orchestrator

**Build:** `evaluate/matcher.rs`, `evaluate/mod.rs`

**Satisfies:** C06, C07, C11, I01, I02, I05, I06, I12

**Gate:** Bundle matched against rules in order. First matching `when` clause wins. Default rule catches unmatched states. No rule match + no default = `E_MISSING_RULE`. `requires` checked before matching = `E_INCOMPLETE_BASIS`. Output `matched_rule` equals winning rule's `name`. Same inputs produce same output.

**Depends on:** D2, D3

### D5 — Output formatting (JSON + human)

**Build:** `output/json.rs`, `output/human.rs`, `output/mod.rs`

**Satisfies:** C08, C10

**Gate:** JSON output validates against `assess.v0` schema (test with `jsonschema` crate). Human output shows decision band, risk factors, basis summary. `pack` fixture `artifact_assess.json` updated to match schema.

**Depends on:** D4

### D6 — Witness integration

**Build:** `witness/record.rs`, `witness/ledger.rs`, `witness/query.rs`, `witness/mod.rs`

**Satisfies:** C09, I10

**Gate:** Witness JSONL appended after every run. `--no-witness` suppresses. `assess witness last` returns the most recent record. `assess witness count` returns total.

**Depends on:** D5

### D7 — operator.json + spine-rules + golden rules

**Build:** `operator.json`, `rules/exit-code-range.yml`, `rules/no-hashmap-in-output.yml`, `rules/witness-must-append.yml`, `tests/golden_rules.rs`

**Satisfies:** C10 (full), non-negotiable #3

**Gate:** `golden_rules.rs` passes. `operator.json` describes the binary, exit codes, refusals, pipeline position (upstream: shape, rvl, verify, benchmark; downstream: pack).

**Depends on:** D6

### D8 — Integration tests + E2E pipeline

**Build:** All test files in `tests/`, all fixtures in `fixtures/`

**Satisfies:** All contracts, all threats, all invariants

**Gate:** Every refusal code exercised. Every decision band exercised. Determinism test passes. Golden output fixtures match byte-for-byte.

**Depends on:** D7

---

## Test matrix

### Unit tests

| Test ID | Contract | Threat | Invariant | Type | Expected result |
|---------|----------|--------|-----------|------|-----------------|
| U01 | C03 | — | — | unit | Valid policy YAML loads into PolicyFile struct |
| U02 | C04 | T01 | — | unit | Missing `schema_version` → `E_BAD_POLICY` |
| U03 | C04 | T01 | — | unit | Empty `rules` list → `E_BAD_POLICY` |
| U04 | C04 | T08 | — | unit | Rule with `condition` field → `E_BAD_POLICY` |
| U05 | C04 | T01 | I06 | unit | Default rule not last → `E_BAD_POLICY` |
| U06 | C04 | — | I08 | unit | Non-PROCEED rule without `risk_code` → `E_BAD_POLICY` |
| U07 | C05 | — | I13 | unit | Artifact `{version: \"shape.v0\"}` resolves canonical tool `shape` |
| U08 | C05 | — | I13 | unit | Artifact `{tool: \"verify\", version: \"verify.report.v1\"}` resolves canonical tool `verify` |
| U09 | C05 | T06 | — | unit | Artifact with no `version` → `E_BAD_ARTIFACT` |
| U10 | C05 | T06 | — | unit | Artifact with malformed explicit `tool` → `E_BAD_ARTIFACT` |
| U11 | C05 | T06 | — | unit | Version `bad_format` (no `.v<N>`) and no explicit `tool` → `E_BAD_ARTIFACT` |
| U12 | C05 | T02 | I04 | unit | One `{tool: \"verify\", version: \"verify.report.v1\"}` and one `{version: \"verify.v0\"}` artifact → `E_DUPLICATE_TOOL` |
| U13 | — | — | I03 | unit | Policy requires `verify`, bundle has only shape + rvl → `E_INCOMPLETE_BASIS` |
| U14 | C06 | — | I05 | unit | Clean bundle matches `clean_reconciliation` (first rule) → PROCEED |
| U15 | C06 | — | I02 | unit | Partial overlap bundle → PROCEED_WITH_RISK, `risk_code: PARTIAL_SCHEMA_OVERLAP` |
| U16 | C06 | — | — | unit | `outcome_in` matches any value in the list |
| U17 | C06 | — | — | unit | `refusal` matches on `refusal.code` field |
| U18 | C06 | — | — | unit | `signals` exact-equality match (key + value must both match) |
| U19 | C06 | — | — | unit | `when` clause with tool not in bundle: clause does not match |
| U20 | C07 | T04 | — | unit | No rule matches, no default → `E_MISSING_RULE` |
| U21 | C07 | — | I06 | unit | Default rule catches unmatched state → BLOCK |
| U22 | C12 | — | I07 | unit | Policy SHA-256 in output matches file hash |
| U23 | — | — | I12 | unit | Output `matched_rule` equals winning rule's `name` field |
| U24 | — | — | I11 | unit | Extra artifact (not in `requires`) appears in `epistemic_basis` |
| U25 | — | T10 | — | unit | Artifact with `outcome: null` and no refusal: recorded, may not match any rule |

### Integration tests

| Test ID | Contract | Threat | Invariant | Type | Expected result |
|---------|----------|--------|-----------|------|-----------------|
| E01 | C01, C08 | — | I09 | integration | shape(COMPATIBLE) + rvl(REAL_CHANGE) + verify(PASS) → PROCEED, exit 0 |
| E02 | C01, C08 | — | I09 | integration | rvl(E_DIFFUSE) → ESCALATE, exit 1 |
| E03 | C01, C08 | — | I09 | integration | Default rule fires → BLOCK, exit 2 |
| E04 | C02 | — | I09 | integration | `E_BAD_POLICY` → valid refusal JSON, exit 2 |
| E05 | C02 | T05 | — | integration | `--policy X --policy-id Y` → `E_AMBIGUOUS_POLICY`, exit 2 |
| E06 | C10 | — | — | integration | Output validates against pack's `artifact_assess.json` schema |
| E07 | C09 | — | I10 | integration | Witness record appended after successful run |
| E08 | C09 | — | I10 | integration | `--no-witness` suppresses witness append |
| E09 | C11 | T09 | I01 | integration | Run 10x with same inputs → all outputs byte-identical |
| E10 | C08 | — | — | integration | `--json` produces valid JSON; no `--json` produces human summary |
| E11 | C03 | — | — | integration | Policy resolved from `ASSESS_POLICY_PATH` env var |
| E12 | — | T08 | — | integration | Policy with `condition: "x >= 0.5"` → `E_BAD_POLICY` |
| E13 | C05 | T02 | I13 | integration | `verify.report.v1` artifact plus legacy `benchmark.v0` artifact resolve to `verify` and `benchmark` and match a tournament policy cleanly |

---

## Quality gates

| Gate | Condition | Test strategy |
|------|-----------|---------------|
| **Gate 1: Determinism** | 10 consecutive runs on identical inputs produce byte-identical output | `tests/determinism.rs`: hash output of 10 runs, assert all hashes equal |
| **Gate 2: Refusal completeness** | Every refusal code in the `RefusalCode` enum is exercised by at least one test | `tests/refusal_suite.rs`: one test per variant, `strum::EnumIter` asserts coverage |
| **Gate 3: Schema conformance** | `assess` JSON output validates against `assess.v0` JSON Schema | `tests/output_schema.rs`: `jsonschema` crate validates all golden outputs |
| **Gate 4: Pipeline integration** | `assess` output accepted by `pack seal` as valid member artifact | `tests/e2e_pipeline.rs`: `pack::detect::member_type` identifies output as `assess` artifact |
| **Gate 5: Golden output stability** | Golden fixture files in `fixtures/golden/` match actual output byte-for-byte | `tests/evaluate_rules.rs`: assert_eq on serialized output vs golden file |

---

## Execution commands

```bash
# Build
cargo check -p assess --all-targets
cargo build -p assess

# Lint + format
cargo clippy -p assess --all-targets -- -D warnings
cargo fmt -p assess -- --check

# Test (all)
cargo test -p assess

# Test (targeted)
cargo test -p assess -- golden_rules
cargo test -p assess -- policy_load
cargo test -p assess -- evaluate_rules
cargo test -p assess -- refusal_suite
cargo test -p assess -- determinism
cargo test -p assess -- e2e_pipeline

# Verify gates
cargo test -p assess -- determinism --nocapture
cargo test -p assess -- refusal_suite --nocapture
cargo test -p assess -- output_schema --nocapture

# Pre-commit (UBS equivalent)
cargo check -p assess --all-targets && \
cargo clippy -p assess --all-targets -- -D warnings && \
cargo test -p assess && \
cargo fmt -p assess -- --check
```

---

## Candidate crates

| Need | Crate | Notes |
|------|-------|-------|
| CLI | `clap` (derive) | Consistent with all spine tools |
| YAML parsing | `serde_yaml` | Policy file loading |
| JSON parsing | `serde_json` | Artifact loading |
| JSON Schema validation | `jsonschema` | Dev-dep: meta-validation of policy files and output |
| Hashing (policy content) | `sha2` | SHA-256 for policy content hash |
| Hashing (witness) | `blake3` | Consistent with spine witness convention |
| Timestamps | `chrono` | Witness records (ISO 8601) |
| Enum utilities | `strum` | Dev-dep: `EnumIter` for refusal coverage tests |
| Temp files | `tempfile` | Dev-dep: test isolation |
| Golden rules | `spine-rules` (git) | Dev-dep: golden rule enforcement |

v0 does not need expression evaluation crates. Match surface is exact equality only.

---

## Determinism

Same artifacts + same policy = same decision. No randomness, no side effects. The policy file is content-hashed and included in the output. No `HashMap` in any output struct. No timestamps in the decision output (timestamps live in witness records only).
