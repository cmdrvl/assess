# assess — Decision Framing

## One-line promise
**Classify structured reports into deterministic decision bands — PROCEED, PROCEED_WITH_RISK, ESCALATE, or BLOCK — using versioned, declared policy rules.**

---

## Problem

After `shape`, `rvl`, `verify`, and `benchmark` produce their reports, someone (or something) needs to decide: do we continue, escalate, or stop?

Today this is ad-hoc agent logic — every pipeline invents its own "is this good enough?" threshold. `assess` replaces that with declared, versioned, deterministic policy. Humans review policies, not ad-hoc decisions.

### Core principle

Truth remains binary. Decisions are graded. `assess` does not change what is true — it classifies results into actionability bands using declared, deterministic rules.

---

## Non-goals

`assess` is NOT:
- A truth tool (that's `rvl`, `verify`, `shape`)
- A diff tool (that's `compare`)
- A scoring tool (that's `benchmark`)
- Probabilistic or ML-based

It does not produce facts. It classifies facts into decisions.

---

## CLI

```
assess <ARTIFACT>... --policy <POLICY> [OPTIONS]

Arguments:
  <ARTIFACT>...          Spine reports to assess (shape, rvl, verify, verify cross, benchmark, compare, fingerprint results)

Options:
  --policy <PATH|ID>     Decision policy file or ID
  --policy-id <ID>       Policy ID (resolved from search path)
```

At least one artifact and a policy are required. `--policy` and `--policy-id` are mutually exclusive; providing both is a refusal (`E_AMBIGUOUS_POLICY`).

### Policy resolution order

1. `ASSESS_POLICY_PATH` env var (colon-separated directories)
2. Built-in policies bundled with the binary
3. `~/.cmdrvl/policies/` if present

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
  "decision_band": "PROCEED_WITH_RISK",
  "confidence_floor": 0.48,
  "policy": {
    "id": "loan_tape.monthly.v1",
    "version": 1,
    "sha256": "sha256:a1b2c3..."
  },
  "risk_factors": [
    {
      "code": "PARTIAL_SCHEMA_OVERLAP",
      "source_tool": "shape",
      "detail": "shape INCOMPATIBLE with 48% column overlap (above policy floor of 45%)"
    }
  ],
  "epistemic_basis": [
    { "artifact": "shape.report.json", "version": "shape.v0", "outcome": "INCOMPATIBLE", "decision_band": "PROCEED_WITH_RISK" },
    { "artifact": "rvl.report.json", "version": "rvl.v0", "outcome": "REAL_CHANGE", "decision_band": "PROCEED" },
    { "artifact": "verify.report.json", "version": "verify.v0", "outcome": "PASS", "decision_band": "PROCEED" }
  ],
  "refusal": null
}
```

---

## Policy file schema (`policy.v0`)

```yaml
schema_version: 1
policy_id: loan_tape.monthly.v1
policy_version: 1
description: "Monthly loan tape reconciliation policy"

rules:
  - name: shape_compatible
    if:
      tool: shape
      outcome: COMPATIBLE
    then:
      decision_band: PROCEED

  - name: partial_overlap_acceptable
    if:
      tool: shape
      outcome: INCOMPATIBLE
      condition: "checks.schema_overlap.overlap_ratio >= 0.45"
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: PARTIAL_SCHEMA_OVERLAP
      confidence_metric: "checks.schema_overlap.overlap_ratio"

  - name: rvl_real_change
    if:
      tool: rvl
      outcome: REAL_CHANGE
    then:
      decision_band: PROCEED

  - name: rvl_no_real_change
    if:
      tool: rvl
      outcome: NO_REAL_CHANGE
    then:
      decision_band: PROCEED

  - name: diffuse_requires_review
    if:
      tool: rvl
      refusal: E_DIFFUSE
    then:
      decision_band: ESCALATE
      risk_code: DIFFUSE_CHANGE

  - name: moderate_missingness
    if:
      tool: rvl
      refusal: E_MISSINGNESS
      condition: "refusal.detail.missing_old < 0.8 and refusal.detail.missing_new < 0.8"
    then:
      decision_band: PROCEED_WITH_RISK
      risk_code: MISSINGNESS_TOLERATED

  - name: verify_fail_requires_review
    if:
      tool: verify
      outcome: FAIL
    then:
      decision_band: ESCALATE
      risk_code: VERIFY_FAIL

  - name: verify_pass
    if:
      tool: verify
      outcome: PASS
    then:
      decision_band: PROCEED

  - name: default_block
    default: true
    then:
      decision_band: BLOCK
      risk_code: UNHANDLED_CONDITION
```

---

## Tool name derivation

Spine tool outputs carry a `version` field (e.g., `rvl.v0`, `verify_cross.v0`, `benchmark.v0`) but no explicit `tool` field. `assess` derives the tool name by stripping the `.v<N>` suffix from the `version` field: `shape.v0` -> `shape`, `verify_cross.v0` -> `verify_cross`. Non-spine reports with an explicit `tool` field use that directly.

## Rule evaluation

- Each input artifact is evaluated independently against the rule set
- For each artifact: rules are evaluated in order; the first rule whose `tool` matches and whose `outcome`/`refusal`/`condition` matches wins
- The overall `decision_band` is the **most restrictive** across all per-artifact decisions: BLOCK > ESCALATE > PROCEED_WITH_RISK > PROCEED
- All per-artifact decisions are recorded in `epistemic_basis`; risk factors from non-PROCEED decisions are collected in `risk_factors`
- `condition` fields are simple expressions evaluated against the artifact's JSON output (dot-path access, comparison operators: `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`)
- `default: true` marks the fallback rule (strongly recommended, must be last)
- All rules must specify `decision_band`; `risk_code` is required for non-PROCEED bands
- If an artifact's tool doesn't match any rule (and no default exists), `assess` refuses with `E_MISSING_RULE`

## confidence_floor

When multiple artifacts are assessed, `confidence_floor` is the minimum per-artifact score:
- PROCEED -> 1.0
- PROCEED_WITH_RISK -> the value of the `confidence_metric` field declared in the matching policy rule, evaluated as a dot-path into the artifact's JSON and clamped to `[0, 1]`. If no `confidence_metric`, defaults to 0.5
- ESCALATE -> 0.0
- BLOCK -> 0.0

The floor is `min()` across all per-artifact scores. It answers "how clean is the weakest link in the epistemic basis?"

---

## Extensibility

`assess` does not hardcode which tools it understands. Policy rules match on the `tool` field in the input JSON. Any structured report with a `version`, `outcome`, and/or `refusal` field can be assessed. For example, a factory conflict resolution engine might emit:

```json
{
  "tool": "conflict",
  "outcome": "DISPUTED",
  "detail": { "column": "noi", "competing_values": 2, "tolerance_exceeded": true }
}
```

A policy can reference this:

```yaml
  - name: conflict_unresolved
    if:
      tool: conflict
      outcome: DISPUTED
      condition: "detail.tolerance_exceeded == true"
    then:
      decision_band: ESCALATE
      risk_code: CONFLICT_UNRESOLVED
```

---

## `assess replay` — Policy Backtesting

### One-line promise

**Re-run a policy against historical evidence packs to see what would have changed — before deploying.**

### Problem

Policies evolve. Thresholds tighten, new rules appear, old rules get removed. Today, changing a policy is a leap of faith — you edit the YAML, deploy it, and discover the consequences in production. `assess replay` eliminates this by backtesting a policy against every historical evidence pack, showing exactly which past decisions would flip.

This is the equivalent of backtesting a trading strategy against historical data. You never deploy a strategy without backtesting; you should never deploy a policy without replaying.

### CLI

```
assess replay --policy <POLICY> --packs <GLOB|DIR> [OPTIONS]

Options:
  --policy <PATH|ID>     New/candidate policy to test
  --policy-id <ID>       Policy ID (resolved from search path)
  --packs <GLOB|DIR>     Evidence packs to replay against (glob or directory)
  --compare              Compare replay decisions against original decisions stored in packs
  --filter-tool <TOOL>   Only replay packs containing artifacts from this tool
  --json                 JSON output (default: human summary)
```

`--packs` accepts a glob pattern (`evidence/2025-*/*.pack`) or a directory (recurse for `.pack` files). Each pack must contain at least one assessable artifact (tool reports with `version`/`outcome`/`refusal` fields).

`--compare` requires that each pack contains an original `assess` decision (a file with `"version": "assess.v0"`). Packs without an original decision are skipped with a warning.

### Exit codes

`0` no flips detected (or `--compare` not used) | `1` flips detected | `2` refusal

### Output (JSON)

```json
{
  "version": "assess_replay.v0",
  "policy": {
    "id": "loan_tape.monthly.v2",
    "version": 2,
    "sha256": "sha256:d4e5f6..."
  },
  "packs_scanned": 24,
  "packs_assessed": 22,
  "packs_skipped": 2,
  "results": [
    {
      "pack": "evidence/2025-11/pack-a7f3b2.pack",
      "pack_sha256": "sha256:...",
      "decision_band": "PROCEED_WITH_RISK",
      "confidence_floor": 0.48,
      "risk_factors": [
        { "code": "PARTIAL_SCHEMA_OVERLAP", "source_tool": "shape" }
      ]
    },
    {
      "pack": "evidence/2025-10/pack-b8c4d3.pack",
      "pack_sha256": "sha256:...",
      "decision_band": "PROCEED",
      "confidence_floor": 1.0,
      "risk_factors": []
    }
  ],
  "summary": {
    "PROCEED": 18,
    "PROCEED_WITH_RISK": 3,
    "ESCALATE": 1,
    "BLOCK": 0
  },
  "compare": null,
  "refusal": null
}
```

### `--compare` output

When `--compare` is provided, the output includes a `compare` block showing how the new policy's decisions differ from the original decisions stored in each pack:

```json
{
  "compare": {
    "flips": 3,
    "unchanged": 19,
    "details": [
      {
        "pack": "evidence/2025-09/pack-c1d2e3.pack",
        "original_band": "PROCEED",
        "replay_band": "PROCEED_WITH_RISK",
        "direction": "stricter",
        "new_risk_factors": [
          { "code": "PARTIAL_SCHEMA_OVERLAP", "source_tool": "shape" }
        ]
      },
      {
        "pack": "evidence/2025-07/pack-f4a5b6.pack",
        "original_band": "ESCALATE",
        "replay_band": "PROCEED_WITH_RISK",
        "direction": "more_lenient",
        "removed_risk_factors": [
          { "code": "DIFFUSE_CHANGE", "source_tool": "rvl" }
        ]
      },
      {
        "pack": "evidence/2025-06/pack-d7e8f9.pack",
        "original_band": "PROCEED",
        "replay_band": "BLOCK",
        "direction": "stricter",
        "new_risk_factors": [
          { "code": "UNHANDLED_CONDITION", "source_tool": "benchmark" }
        ]
      }
    ]
  }
}
```

Flip `direction` values:
- `stricter` — the new policy produces a more restrictive band (PROCEED → PROCEED_WITH_RISK, PROCEED → BLOCK, etc.)
- `more_lenient` — the new policy produces a less restrictive band (ESCALATE → PROCEED_WITH_RISK, BLOCK → PROCEED, etc.)

Band ordering for direction: PROCEED < PROCEED_WITH_RISK < ESCALATE < BLOCK.

### Human output

Without `--json`, `assess replay` prints a compact summary:

```
Policy: loan_tape.monthly.v2 (sha256:d4e5f6...)
Packs:  22 assessed, 2 skipped

  PROCEED            18  ████████████████████████████████████  82%
  PROCEED_WITH_RISK   3  █████                                 14%
  ESCALATE            1  ██                                     5%
  BLOCK               0                                         0%

Flips vs original (--compare):
  ⬆ stricter     2  (PROCEED → PROCEED_WITH_RISK, PROCEED → BLOCK)
  ⬇ more_lenient 1  (ESCALATE → PROCEED_WITH_RISK)
  = unchanged   19
```

### Refusal codes (replay-specific)

| Code | Trigger | Next step |
|------|---------|-----------|
| `E_NO_PACKS` | No packs found matching glob | Check path/glob |
| `E_PACK_UNREADABLE` | Can't read or parse pack | Check pack integrity |
| `E_NO_ARTIFACTS` | Pack contains no assessable artifacts | Check pack contents |
| `E_NO_ORIGINAL` | `--compare` used but pack has no original decision | Remove `--compare` or use packs with decisions |

Plus all standard `assess` refusal codes (`E_BAD_POLICY`, `E_AMBIGUOUS_POLICY`, etc.).

### Usage examples

```bash
# Backtest a candidate policy against all 2025 evidence
assess replay --policy loan_tape.monthly.v2 \
  --packs "evidence/2025-*/" --json

# Compare against original decisions to find flips
assess replay --policy loan_tape.monthly.v2 \
  --packs "evidence/2025-*/" --compare

# Test only packs that include shape reports
assess replay --policy loan_tape.monthly.v2 \
  --packs evidence/ --filter-tool shape --compare

# CI gate: fail if policy change would flip any historical decision
assess replay --policy candidate.yaml --packs evidence/ --compare
# exit code 1 if flips detected → PR reviewer sees the impact
```

### Why this matters

1. **Testable policies.** Policies become code with a test suite — the evidence archive. You don't guess what a rule change does; you measure it.
2. **Composes with `pack`.** Evidence packs are content-addressed and tamper-evident. `replay` inherits this: the inputs to replay are provably the same data that produced the original decisions.
3. **Agent-native.** An agent tightening a policy threshold can run `assess replay --compare` before committing the change — a self-check before the policy goes live.
4. **CI integration.** `exit code 1` on flips means a policy PR can be gated on historical impact analysis. Reviewers see "this change would have blocked 3 of 24 past reconciliations" before merging.
5. **Deterministic.** Same packs + same candidate policy = same replay results. The replay itself is reproducible and auditable.

---

## Refusal codes

| Code | Trigger | Next step |
|------|---------|-----------|
| `E_BAD_POLICY` | Policy file invalid | Fix policy syntax |
| `E_AMBIGUOUS_POLICY` | Both `--policy` and `--policy-id` provided | Use one |
| `E_UNKNOWN_POLICY` | Policy ID not found | Check policy path |
| `E_BAD_ARTIFACT` | Can't parse input artifact | Check artifact format |
| `E_MISSING_RULE` | No rule matched (no default) | Add default rule to policy |

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

# assess -> pack: decision becomes part of evidence
assess shape.json rvl.json verify.json --policy loan_tape.monthly.v1 > decision.json
pack seal shape.json rvl.json verify.json decision.json nov.lock.json dec.lock.json \
  --note "Nov->Dec recon with decision" --output evidence/2025-12/
```

---

## Implementation notes

### Candidate crates

| Need | Crate | Notes |
|------|-------|-------|
| Expression evaluation (v0) | `evalexpr` | Simple condition expressions (`>= 0.45`, `and`/`or`) |
| Expression evaluation (v1) | `cel-interpreter` | Google CEL — native dot-path access, `has()`, guaranteed termination |
| YAML parsing | `serde_yaml` | Policy file loading |
| JSON Schema validation | `jsonschema` | Meta-validation of policy files |
| JSON parsing | `serde_json` | Artifact loading |

### Expression evaluation path

v0: `evalexpr` handles simple comparisons. Requires flattening JSON to a variable context (~50 LOC wrapper).

v1: `cel-interpreter` (Google's Common Expression Language) is the better semantic fit — natively supports dot-path access on maps, `has()` for optional fields, guaranteed termination, no side effects. CEL expressions look like: `checks.schema_overlap.overlap_ratio >= 0.45`. Migration path is clean: simple comparisons are valid in both.

---

## Determinism

Same artifacts + same policy = same decision. No randomness, no side effects. The policy file is content-hashed and included in the output.
