# AGENTS.md — assess

> Repo-specific guidance for AI coding agents working in `assess`.

This file adds repo-specific instructions on top of the shared monorepo rules when you are working inside the full `cmdrvl` workspace. In the standalone `assess` repo, treat this file and [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md) as the local source of truth.

---

## assess — What This Project Does

`assess` is the epistemic spine's **decision primitive**.

It evaluates a complete evidence bundle against a declared policy and emits one deterministic decision artifact.

Pipeline position:

```text
shape / rvl / verify / benchmark -> assess -> pack
```

What `assess` owns:

- policy loading and validation
- epistemic-basis completeness checks
- ordered rule matching
- deterministic decision output
- refusal envelopes for unsafe or incomplete invocations
- local witness receipt logging

What `assess` does not own:

- fact production
- diffing
- constraint validation
- gold-set scoring
- entity resolution
- factory winner selection

If the work sounds like scoring, validation, or comparison, it probably belongs in another repo.

---

## Current Repository State

This repo now contains the scaffolded Rust crate plus the implementation Beads graph.

Current contents:

- [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md) — implementation-grade spec
- [.beads/issues.jsonl](./.beads/issues.jsonl) — execution graph
- [README.md](./README.md) — operator-facing framing
- scaffolded crate tree with module boundaries, schemas, rules, fixtures, and named test layout

Important reality check:

- full assessment semantics are not implemented yet
- metadata surfaces are implemented (`--describe`, `--schema`, `--version`)
- decision execution and witness execution are still active implementation work

Implication:

- keep the implementation aligned to the plan
- do not smuggle in “reasonable” semantics that the plan does not declare
- do not collapse module boundaries into `main.rs`

---

## Quick Reference

```bash
# Read the spec first
sed -n '1,260p' docs/PLAN_ASSESS.md

# Beads / graph
br ready
br blocked
br show <id>

# Graph-aware prioritization
bv --robot-next
bv --robot-triage
bv --robot-plan

# Current crate verification
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
ubs .

# Metadata surfaces already live
cargo run -- --describe
cargo run -- --schema
cargo run -- --version
```

Docs-only changes:

```bash
git diff --check
ubs --diff
```

---

## Source of Truth

- **Spec:** [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md)
- **Execution graph:** [.beads/issues.jsonl](./.beads/issues.jsonl)

If code, README, and plan disagree, the plan wins.

Do not revive stale `compare` ideas or ad hoc policy logic. `assess` is a narrow, deterministic decision classifier.

---

## Current File Map

| Path | Purpose |
|------|---------|
| `Cargo.toml` | crate root |
| `src/main.rs` | thin binary entrypoint only |
| `src/lib.rs` | module tree and top-level execution surface |
| `src/cli/args.rs` | clap structs and command wiring |
| `src/cli/exit.rs` | exit-code model |
| `src/policy/schema.rs` | policy data model |
| `src/policy/loader.rs` | policy resolution order |
| `src/policy/validate.rs` | policy validation helpers |
| `src/bundle/artifact.rs` | artifact basis model |
| `src/bundle/derive.rs` | canonical tool derivation |
| `src/evaluate/matcher.rs` | rule matching surface |
| `src/evaluate/mod.rs` | decision model/orchestrator surface |
| `src/output/json.rs` | JSON rendering surface |
| `src/output/human.rs` | human rendering surface |
| `src/refusal/codes.rs` | refusal code set |
| `src/refusal/payload.rs` | refusal envelope model |
| `src/witness/record.rs` | witness record schema |
| `src/witness/ledger.rs` | witness append surface |
| `src/witness/query.rs` | witness query modes |
| `schemas/*.json` | schema contracts |
| `rules/*.yml` | golden-rule enforcement artifacts |
| `fixtures/**` | shared policy/artifact/golden fixtures |
| `tests/support/**` | shared test helpers only |
| `tests/*.rs` | named suites matching the plan |

Critical structural rules:

- `src/main.rs` stays thin
- top-level command behavior belongs in `src/lib.rs` + `src/cli/**`
- real decision semantics belong in `src/evaluate/**`
- refusal shape belongs in `src/refusal/**`
- witness stays local receipt logging only

Do not move the whole product into one file just because the repo is still young.

---

## Core Invariants (Do Not Break)

### 1. Determinism is constitutional

Same artifacts plus same policy must yield the same decision bytes.

No timestamps, no random ordering, no ambient state in decision output.

### 2. No expression engine in v0

v0 policy matching is exact equality only.

Do not add:

- CEL
- evalexpr
- numeric threshold DSL
- arbitrary JSON path traversal

If a change needs expression semantics, stop and update the plan first.

### 3. Every input is accounted for

Every artifact provided at the CLI must appear in `epistemic_basis`.

Extra artifacts may be irrelevant to a given policy, but they cannot disappear silently.

### 4. Ordered rule matching only

Rules are evaluated in declaration order.

First match wins. Default rule must be last.

### 5. assess is not a scorer

`benchmark` owns scoring. `verify` owns constraints.

`assess` consumes those artifacts and classifies them into a decision band.

### 6. Witness is local only

Witness records are local receipt logs.

Do not turn `assess` witness output into a portable evidence artifact or make it authoritative over `pack` or `lock`.

### 7. Refusals are protocol surface

Do not replace structured refusal envelopes with ad hoc human text.

The refusal contract is part of the tool.

---

## Toolchain

- language: Rust
- package manager: Cargo only
- edition: 2024
- unsafe code: forbidden

Current dependencies are intentionally small:

- `clap`
- `serde`
- `serde_json`
- `thiserror`

Do not add heavy dependency layers casually during early implementation.

---

## Quality Gates

Run after substantive code changes:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
ubs .
```

Current named suite surface:

- `tests/golden_rules.rs`
- `tests/policy_load.rs`
- `tests/bundle_construct.rs`
- `tests/evaluate_rules.rs`
- `tests/refusal_suite.rs`
- `tests/output_schema.rs`
- `tests/witness_suite.rs`
- `tests/e2e_pipeline.rs`
- `tests/determinism.rs`

The scaffold tests are intentionally lightweight today. They should become real behavioral gates as the implementation lands.

---

## Beads Workflow

The implementation graph is already present. Use it.

Typical loop:

```bash
br ready
br show <id>
br update <id> --status=in_progress
```

Current post-scaffold graph shape:

- early contract lanes: refusal contract, schema/bundle types
- shared resource lanes: fixture corpus, test-support harness
- infrastructure lane: CI / release prerequisites
- later semantic lanes: policy loader, bundle parsing, evaluator, rendering, integration

Do not claim the epic. Claim real child beads only.

---

## MCP Agent Mail — Multi-Agent Coordination

Agent Mail is the coordination layer for multi-agent sessions in this repo: identities, inbox/outbox, searchable threads, and advisory file reservations.

### Session Baseline

1. If direct MCP Agent Mail tools are available in this harness, ensure project and reuse your identity:
   - `ensure_project(project_key=<abs-path>)`
   - `whois(project_key, agent_name)` or `register_agent(...)` only if identity does not exist
2. Reserve only exact files you will edit:
   - Allowed: `src/policy/schema.rs`, `tests/evaluate_rules.rs`
   - Not allowed: `src/**`, `tests/**`, whole directories
3. Send a short start message and finish message for each bead, reusing the bead ID as the thread when practical.
4. Check inbox at moderate cadence, not continuously.

### Important `ntm` Boundary

When this repo is worked via `ntm`, the session may be connected to Agent Mail even if the spawned Codex or Claude harness does **not** expose direct `mcp__mcp-agent-mail__...` tools.

If direct MCP Agent Mail tools are unavailable:

- do **not** stop working just because mail tools are absent
- continue with `br`, exact file reservations via the available coordination surface, and overseer instructions
- treat Beads + narrow file ownership as the minimum coordination contract

### Communication Rules

- If a message has `ack_required=true`, acknowledge it promptly.
- Keep bead updates short and explicit: start message, finish message, blocker message.
- Reuse a stable bead thread when possible for searchable history.

---

## Current Implementation Guidance

Right now the important thing is to preserve the scaffold’s honesty:

- metadata surfaces can be real
- placeholder semantics must be explicit
- do not fake successful assess decisions before policy/evaluator lanes are built

The current scaffold already gives you these safe boundaries:

- metadata paths are live
- evaluation paths can still refuse cleanly
- tests already prove the tree compiles and the embedded artifacts exist

Build on that. Do not paper over missing semantics with fake outputs.
