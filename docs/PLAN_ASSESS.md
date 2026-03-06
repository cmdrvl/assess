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
