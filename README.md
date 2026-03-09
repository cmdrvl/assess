# assess

**Deterministic decision classification over a complete spine evidence bundle.**

`assess` is the epistemic spine tool that turns a complete set of upstream evidence artifacts into one declared decision:

- `PROCEED`
- `PROCEED_WITH_RISK`
- `ESCALATE`
- `BLOCK`

It answers one narrow question:

**Given this policy and this evidence bundle, what action band should we assign?**

Current status:

- repository status: scaffolded Rust crate with module skeleton, fixture corpus, schema artifacts, and named test layout
- source of truth: [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md)
- current implementation state: metadata surfaces are live (`--describe`, `--schema`, `--version`), but full policy evaluation and witness execution are not implemented yet

This repo is now ready for the main implementation swarm. The plan and Beads graph are already in place; the crate tree is no longer docs-only.

---

## Current quickstart

Contributor and local-operator quickstart:

```bash
cd assess
sed -n '1,260p' docs/PLAN_ASSESS.md
br ready
cargo run -- --help
cargo run -- --describe
cargo run -- --schema
cargo run -- --version
cargo test
```

At the moment:

- `--describe`, `--schema`, and `--version` are implemented
- the full `assess <ARTIFACT>... --policy ...` command path is scaffolded but not semantically complete yet
- witness subcommands are scaffolded in the CLI tree but not fully implemented yet

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

- policy loading and validation
- complete-bundle basis checks
- deterministic ordered rule matching
- one decision artifact per invocation
- refusal envelopes for unsafe or incomplete assessment attempts
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

## Current repository shape

The scaffold now matches the plan’s module layout:

- [Cargo.toml](./Cargo.toml)
- [src/lib.rs](./src/lib.rs)
- [src/main.rs](./src/main.rs)
- [src/cli/](./src/cli)
- [src/policy/](./src/policy)
- [src/bundle/](./src/bundle)
- [src/evaluate/](./src/evaluate)
- [src/output/](./src/output)
- [src/refusal/](./src/refusal)
- [src/witness/](./src/witness)
- [schemas/](./schemas)
- [rules/](./rules)
- [fixtures/](./fixtures)
- [tests/](./tests)

The current scaffold rule is important:

- `src/main.rs` stays thin
- `src/lib.rs` owns the module tree and top-level execution surface
- placeholder behavior should stay explicit rather than pretending evaluation is already implemented

---

## Current v0 contract direction

Target CLI surface:

```text
assess <ARTIFACT>... --policy <POLICY> [OPTIONS]
assess witness <query|last|count> [OPTIONS]
```

Target domain outcomes:

| Exit | Meaning |
|------|---------|
| `0` | `PROCEED` |
| `1` | `PROCEED_WITH_RISK` or `ESCALATE` |
| `2` | `BLOCK` or refusal |

Important boundary:

- v0 uses exact-equality policy matching only
- no CEL, no expression engine, no numeric-threshold DSL
- same artifacts plus same policy must produce the same decision bytes

Read [docs/PLAN_ASSESS.md](./docs/PLAN_ASSESS.md) for the full contract.

---

## Current execution graph

The implementation graph already exists in Beads:

- [.beads/issues.jsonl](./.beads/issues.jsonl)

Typical workflow:

```bash
br ready
br show <id>
br update <id> --status=in_progress
```

The scaffold bead is already closed. The current open lanes are the real implementation work: refusal contract, shared schema/bundle types, fixture seeding, CI/release setup, and shared test support.

---

## What not to do

Do not turn `assess` into:

- a general JSON rules engine
- a numeric scoring engine
- a diff tool
- a benchmark replacement
- a verify replacement
- a factory resolver

If the desired behavior starts sounding like “score,” “compare,” or “validate,” it probably belongs in another tool.

---

## Release status

`assess` is not release-ready yet.

The repo now has the crate scaffold and implementation graph, but it still needs:

- real policy/bundle/evaluator implementation
- repo-local CI and release workflows
- final README/AGENTS reconciliation after the implementation settles

So treat the current repo as implementation-ready, not release-ready.
