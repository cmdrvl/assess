# assess

![assess: deterministic decision classification. A painterly dashboard showing a complete evidence bundle (delivery.lock, shape.report, rvl.report, fingerprint.catalog, verify.report, canon.mapping) being mapped through a visible policy.yaml file onto a four-band verdict strip. Three bands (PROCEED, ESCALATE, BLOCK) are dimmed; one (PROCEED_WITH_RISK) is lit and chosen. A reason chain below names the rule, the matched evidence, the observed value, and the decision. A WHAT ASSESS DOES NOT DO panel forbids interpretation, policy adjustment, guessing, and override. A triage tag hanging by a string carries four color-coded stickers, with the chosen band peeled off and attached to a delivery folder.](docs/images/assess.webp)

> *Given this policy and this bundle, here's the action.*

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

Install with Homebrew:

```bash
brew install cmdrvl/tap/assess
```

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

Run the article claims-verifier example bundle:

```bash
./target/release/assess \
  fixtures/artifacts/benchmark_claims_high.json \
  fixtures/artifacts/article_verify_pass.json \
  fixtures/artifacts/decoding_claims_converged.json \
  --policy fixtures/policies/claims_verifier_graam_v1.yaml \
  --json
```

This fixture shows the non-standard tool path used by `cmdrvl-cli`
claims-verifier. `article_verify.v0` and `decoding.spine.v0` both set explicit
top-level `tool` values so policy rules can match the canonical identities
`article_verify` and `decoding`.

Inspect metadata:

```bash
./target/release/assess --describe
./target/release/assess --schema
./target/release/assess --version
./target/release/assess --robot-triage
./target/release/assess capabilities --json
./target/release/assess robot-docs guide
./target/release/assess doctor health --json
./target/release/assess doctor capabilities --json
./target/release/assess doctor robot-docs
./target/release/assess doctor --robot-triage
```

Query the local witness log:

```bash
./target/release/assess witness last --json
./target/release/assess witness query --json
./target/release/assess witness count --json
```

Run read-only doctor diagnostics for agent automation:

```bash
./target/release/assess --robot-triage
./target/release/assess capabilities --json
./target/release/assess robot-docs guide
./target/release/assess doctor health --json
./target/release/assess doctor capabilities --json
./target/release/assess doctor --robot-triage
```

`assess doctor` does not read artifacts or policy files, construct bundles,
evaluate rules, query or append witness ledgers, write `.doctor/` artifacts, or
contact providers. `assess doctor --fix` is intentionally unsupported.
The top-level agent entrypoints are read-only aliases over the same doctor
contract; they exist so an agent can discover the CLI without remembering the
doctor namespace first. Bare `assess` prints long help and exits 0.

Configuration footprint:

- User policy fallback: `~/.cmdrvl/config/assess/policies/`
- Shared witness ledger fallback: `~/.cmdrvl/state/witness/witness.jsonl`
- Explicit overrides: `ASSESS_POLICY_PATH` for policy search paths and
  `EPISTEMIC_WITNESS` for a specific witness ledger
- First use copies legacy `~/.epistemic/policies/` and
  `~/.epistemic/witness.jsonl` into the canonical locations, leaves legacy
  files in place, and writes path-only records to
  `~/.cmdrvl/migrations/applied.jsonl` and
  `~/.cmdrvl/notices/deprecated-paths.jsonl`

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
- `assess --json`: read-only capabilities JSON when no artifacts are supplied
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
2. `--policy-id <id>` searches `ASSESS_POLICY_PATH` directories, builtin policies, and `~/.cmdrvl/config/assess/policies/` for a matching `policy_id`

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

The fixture set includes both the canonical loan-tape bundle and the
claims-verifier bundle. The latter demonstrates that `assess` can classify
spine-compatible reports from upstream tools outside the original
`shape`/`rvl`/`verify` trio when those reports expose `version`,
`policy_signals`, and an explicit canonical `tool`.

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

---

*`assess` is part of the open-source toolchain from the [CMD+RVL](https://cmdrvl.com) lineage and AI enablement practice. MIT-licensed. Contributions welcome from any practice or stack.*
