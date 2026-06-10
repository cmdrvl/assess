# assess Ergonomics Handoff

Completed pass 1 in full mode.

Applied:

- bare `assess` prints long help and exits 0
- `assess --robot-triage`
- `assess capabilities --json`
- `assess robot-docs guide`
- CLI error breadcrumbs for JSON typos and usage errors
- audit-clean generated Homebrew formula

Preflight caveat: macOS environment did not provide `flock`; work proceeded
serially. No source decision semantics changed.

Required before release: run the Rust quality gate, audit regression scripts,
`validate_pass.sh`, UBS, then push `main` and verify the workflow-created release
and Homebrew formula.
