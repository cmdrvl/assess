# assess

**Deterministic decision classification over a spine evidence bundle.**

`assess` is the epistemic spine tool that turns a complete set of upstream evidence artifacts into one declared decision:

- `PROCEED`
- `PROCEED_WITH_RISK`
- `ESCALATE`
- `BLOCK`

It answers one narrow question:

**Given this policy and this evidence bundle, what action band should we assign?**

---

## Quickstart

Build from source:

```bash
cargo build --release
./target/release/assess --help
```

Run a decision against a policy:

```bash
./target/release/assess \
  fixtures/artifacts/shape_compatible.json \
  fixtures/artifacts/rvl_real_change.json \
  fixtures/artifacts/verify_pass.json \
  --policy fixtures/policies/loan_tape_monthly_v1.yaml \
  --json
```

Use a policy ID to resolve from the search path:

```bash
./target/release/assess \
  fixtures/artifacts/shape_compatible.json \
  fixtures/artifacts/rvl_real_change.json \
  fixtures/artifacts/verify_pass.json \
  --policy-id loan_tape.monthly.v1 \
  --json
```

Scan outcomes without opening the full JSON artifact:

```bash
./target/release/assess \
  fixtures/artifacts/shape_compatible.json \
  fixtures/artifacts/rvl_real_change.json \
  fixtures/artifacts/verify_pass.json \
  --policy-id loan_tape.monthly.v1 \
  --render summary

./target/release/assess \
  fixtures/artifacts/shape_compatible.json \
  fixtures/artifacts/rvl_real_change.json \
  fixtures/artifacts/verify_pass.json \
  --policy-id loan_tape.monthly.v1 \
  --render summary-tsv
```

Inspect metadata:

```bash
./target/release/assess --describe
./target/release/assess --schema
./target/release/assess --version
```

Query the local witness log:

```bash
./target/release/assess witness last --json
./target/release/assess witness query --json
./target/release/assess witness count --json
```

---

## Exit Codes

| Exit | Meaning |
|------|---------|
| `0` | `PROCEED` |
| `1` | `PROCEED_WITH_RISK` or `ESCALATE` |
| `2` | `BLOCK`, refusal, or CLI error |

---

## Output Modes

- default: compact human-readable decision or refusal report
- `--json`: canonical `assess.v0` JSON artifact or structured refusal envelope
- `--render summary`: one-line operator summary with decision/refusal, matched rule, risk code, tools, witness state, and refusal code
- `--render summary-tsv`: stable header + row TSV summary for shell pipelines

---

## Why assess exists

The upstream spine tools each answer different questions:

- `shape`: can these artifacts be compared?
- `rvl`: what materially changed?
- `verify`: did declared constraints hold?
- `benchmark`: did the extracted facts match gold truth?

But after those reports exist, something still has to decide what to do next.

`assess` exists so that decision is:

- declared
- versioned
- deterministic
- reviewable as policy instead of ad hoc agent logic

`assess` does not change truth. It classifies already-produced evidence into action bands.

---

## What assess owns

- policy loading, validation, and content-addressed hashing
- complete-bundle epistemic-basis checks
- deterministic ordered rule matching (first match wins)
- one decision artifact per invocation
- structured refusal envelopes for unsafe or incomplete assessment attempts
- local witness receipt logging

## What assess does not own

- fact production
- structural comparison
- diffing
- business-rule validation
- gold-set scoring
- entity resolution
- factory winner selection

That means:

- `verify` stays the constraint primitive
- `benchmark` stays the scoring primitive
- `assess` stays the decision primitive

---

## Where assess fits

`assess` sits after the evidence-producing tools and before sealing:

```text
shape / rvl / verify / benchmark -> assess -> pack
```

Related tools:

| If you need... | Use |
|----------------|-----|
| Structural comparability | `shape` |
| Material delta analysis | `rvl` |
| Constraint validation | `verify` |
| Gold-set correctness scoring | `benchmark` |
| Evidence sealing | `pack` |

Use `assess` when the question is:

**Is this evidence bundle good enough to proceed, risky enough to annotate, uncertain enough to escalate, or unsound enough to block?**

---

## Policies

Policies are YAML files conforming to the `policy.v0` schema (`schemas/policy.v0.schema.json`).

A policy declares:

- `requires`: which upstream tools must be present in the evidence bundle
- `rules`: an ordered list of condition/action pairs (first match wins)
- `default`: exactly one rule must be marked as the default fallback

v0 uses exact-equality matching only. No CEL, no expression engine, no numeric-threshold DSL.

Policy resolution:

1. `--policy <path>` loads a policy from a file path
2. `--policy-id <id>` searches `ASSESS_POLICY_PATH` directories and `rules/` for a matching `policy_id`

---

## Refusal Codes

When assess cannot produce a valid decision, it emits a structured refusal envelope:

| Code | Meaning |
|------|---------|
| `E_BAD_POLICY` | Policy file is malformed YAML or fails schema validation |
| `E_AMBIGUOUS_POLICY` | Both `--policy` and `--policy-id` were specified |
| `E_UNKNOWN_POLICY` | Policy ID could not be resolved from search paths |
| `E_BAD_ARTIFACT` | An artifact file could not be read or parsed as JSON |
| `E_DUPLICATE_TOOL` | Multiple artifacts claim the same upstream tool |
| `E_INCOMPLETE_BASIS` | Required tools are missing from the evidence bundle |
| `E_MISSING_RULE` | No rule matched the evidence (should not happen with a default rule) |

---

## Repository Structure

| Path | Purpose |
|------|---------|
| `src/main.rs` | Thin binary entrypoint |
| `src/lib.rs` | Module tree and top-level execution surface |
| `src/cli/` | CLI argument parsing, routing, exit-code model |
| `src/policy/` | Policy loading, validation, schema types |
| `src/bundle/` | Artifact loading, basis derivation |
| `src/evaluate/` | Rule matching and decision orchestration |
| `src/output/` | Deterministic JSON, human, summary, and TSV rendering |
| `src/refusal/` | Refusal codes and envelope model |
| `src/witness/` | Local witness ledger, record schema, query surface |
| `schemas/` | Embedded JSON schemas (`assess.v0`, `policy.v0`) |
| `rules/` | Golden-rule enforcement artifacts |
| `fixtures/` | Policy, artifact, and golden-output fixtures |
| `tests/` | 14 named test suites, 147+ tests |

---

## Quality Gates

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/ubs_gate.sh
```

---

## Core Invariants

1. **Determinism is constitutional.** Same artifacts + same policy = same decision bytes. No timestamps, no random ordering, no ambient state in decision output.
2. **No expression engine in v0.** Policy matching is exact equality only.
3. **Every input is accounted for.** Every artifact appears in `epistemic_basis`.
4. **Ordered rule matching only.** Rules evaluated in declaration order. First match wins. Default rule must be last.
5. **assess is not a scorer.** It classifies evidence into action bands. Scoring belongs in `benchmark`.
6. **Witness is local only.** Witness records are local receipt logs, not portable evidence.
7. **Refusals are protocol surface.** Structured envelopes, not ad hoc text.

---

## Release

Releases are cut automatically via `.github/workflows/release.yml` when `Cargo.toml` version changes on `main`. The workflow builds cross-platform binaries (5 targets), generates SHA256SUMS with cosign signing, SBOM, and SLSA provenance, and publishes to GitHub Releases and the Homebrew tap.
