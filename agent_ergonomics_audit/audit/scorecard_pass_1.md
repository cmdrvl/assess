# assess Scorecard Pass 1

Mode: full.

The pre-pass CLI already had strong decision determinism and read-only doctor
surfaces. The main agent friction was discoverability: the useful surfaces were
behind `doctor`, bare invocation failed, and release formula generation did not
pass strict Homebrew audit.

Post-pass results:

- First-try success: 250 -> 850
- Output parseability: 700 -> 900
- Self-documentation: 620 -> 880
- Error pedagogy: 520 -> 780
- Release hygiene: 400 -> 820

No decision semantics were changed.
