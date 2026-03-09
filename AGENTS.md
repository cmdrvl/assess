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

- policy loading, validation, and content-addressed hashing
- epistemic-basis completeness checks
- ordered rule matching (first match wins, exact equality only)
- deterministic decision output (JSON and human modes)
- structured refusal envelopes for unsafe or incomplete invocations
- local witness receipt logging and query surface

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

This repo contains a fully implemented v0 decision primitive.

- **147+ tests** across 14 named suites
- **All 4 decision bands live**: PROCEED (exit 0), PROCEED_WITH_RISK (exit 1), ESCALATE (exit 1), BLOCK (exit 2)
- **7 refusal codes**: E_BAD_POLICY, E_AMBIGUOUS_POLICY, E_UNKNOWN_POLICY, E_BAD_ARTIFACT, E_DUPLICATE_TOOL, E_INCOMPLETE_BASIS, E_MISSING_RULE
- **Full pipeline**: policy loading → bundle construction → rule evaluation → deterministic output → witness append
- **Quality gates green**: fmt, clippy, test, UBS
- **CI and release workflows**: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- **Determinism proven**: byte-exact determinism across all 4 bands and refusals

---

## Quick Reference

```bash
# Read the spec
sed -n '1,260p' docs/PLAN_ASSESS.md

# Beads / graph
br ready
br blocked
br show <id>

# Quality gates
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/ubs_gate.sh

# Run assess
cargo run -- --help
cargo run -- --describe
cargo run -- --schema
cargo run -- --version

# Full decision pipeline
cargo run -- \
  fixtures/artifacts/shape_compatible.json \
  fixtures/artifacts/rvl_real_change.json \
  fixtures/artifacts/verify_pass.json \
  --policy fixtures/policies/loan_tape_monthly_v1.yaml \
  --json
```

---

## Source of Truth

- **Spec:** [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md)
- **Execution graph:** [.beads/issues.jsonl](./.beads/issues.jsonl)

If code, README, and plan disagree, the plan wins.

Do not revive stale `compare` ideas or ad hoc policy logic. `assess` is a narrow, deterministic decision classifier.

---

## File Map

| Path | Purpose |
|------|---------|
| `Cargo.toml` | Crate root (single crate, not a workspace) |
| `src/main.rs` | Thin binary entrypoint only |
| `src/lib.rs` | Module tree and top-level execution surface |
| `src/cli/args.rs` | Clap structs and command wiring |
| `src/cli/exit.rs` | Exit-code model (0/1/2 trinity) |
| `src/cli/mod.rs` | Route dispatch |
| `src/policy/schema.rs` | Policy data model (PolicyFile, Rule, WhenClause, ToolMatcher) |
| `src/policy/loader.rs` | Policy resolution: file path and ID-based search |
| `src/policy/validate.rs` | Policy structural validation |
| `src/policy/mod.rs` | PolicyError and LoadedPolicy types |
| `src/bundle/artifact.rs` | Artifact basis model and ArtifactBasisEntry |
| `src/bundle/derive.rs` | Canonical tool derivation from artifact JSON |
| `src/bundle/mod.rs` | ArtifactBundle construction and BundleError |
| `src/evaluate/matcher.rs` | Rule matching: WhenClause against ArtifactBundle |
| `src/evaluate/mod.rs` | Decision orchestration, requires check, EvalError |
| `src/output/mod.rs` | AssessOutput, AssessResult, build_output, render dispatch |
| `src/output/json.rs` | Deterministic JSON rendering |
| `src/output/human.rs` | Human-readable rendering |
| `src/refusal/codes.rs` | RefusalCode enum (7 codes) |
| `src/refusal/payload.rs` | RefusalEnvelope model |
| `src/witness/record.rs` | WitnessRecord with builder pattern |
| `src/witness/ledger.rs` | Append-only JSONL ledger at ~/.epistemic/witness.jsonl |
| `src/witness/query.rs` | Witness query/last/count modes |
| `schemas/*.json` | Embedded JSON schemas (assess.v0, policy.v0) |
| `rules/*.yml` | Golden-rule enforcement artifacts (ast-grep) |
| `fixtures/policies/` | Policy YAML fixtures |
| `fixtures/artifacts/` | Artifact JSON fixtures (shape, rvl, verify) |
| `fixtures/golden/` | Golden JSON outputs for all 4 decision bands |
| `tests/support/` | Shared test helpers (TempWorkspace, fixture paths, assertions) |
| `tests/*.rs` | 14 named suites |

Critical structural rules:

- `src/main.rs` stays thin
- top-level command behavior belongs in `src/lib.rs` + `src/cli/**`
- real decision semantics belong in `src/evaluate/**`
- refusal shape belongs in `src/refusal/**`
- witness stays local receipt logging only

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

- Language: Rust
- Package manager: Cargo only
- Edition: 2024
- Unsafe code: forbidden (`#![forbid(unsafe_code)]`)

Dependencies:

- `clap` — CLI framework
- `serde`, `serde_json` — serialization
- `serde_yaml` — policy file parsing
- `sha2` — policy content-addressed hashing
- `thiserror` — error types
- `jsonschema` (dev) — schema validation in tests

---

## Quality Gates

Run after substantive code changes:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/ubs_gate.sh
```

Named test suites (14 suites, 147+ tests):

| Suite | Purpose |
|-------|---------|
| `golden_rules` | Operator manifest contract, ast-grep rule compliance |
| `policy_load` | Policy loading and validation |
| `policy_loader` | Policy ID resolution and search path |
| `bundle_construct` | Artifact bundle construction and basis derivation |
| `evaluate_rules` | Rule matching against bundles |
| `refusal_suite` | Refusal envelope completeness |
| `output_schema` | JSON schema validation, golden file comparison, human output |
| `witness_suite` | Witness record, ledger, and query surface |
| `e2e_pipeline` | Full pipeline integration (all bands + refusals) |
| `determinism` | Byte-exact determinism proof across all bands and refusals |
| `cli_shell` | CLI argument parsing and routing |
| `pack_compat` | Pack-compatible artifact detection |
| `support` | Test harness self-tests |

---

## Beads Workflow

Use the Beads issue tracker for task management:

```bash
br ready           # find unblocked work
br show <id>       # inspect a bead
br update <id> --status=in_progress
br close <id>      # mark complete
br sync --flush-only
```

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
