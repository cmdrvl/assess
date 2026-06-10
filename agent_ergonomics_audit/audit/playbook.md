# assess Agent Ergonomics Playbook

Use `assess --robot-triage` first when the objective is discovery or health.
Use `assess capabilities --json` when generating commands programmatically.
Use `assess robot-docs guide` for a compact human-readable agent guide.

Decision runs remain narrow:

```bash
assess <ARTIFACT>... --policy <PATH> --json
assess <ARTIFACT>... --policy-id <ID> --render summary
```

If a run refuses, inspect `refusal.next_command`; all refusal next commands are
now JSON-ready and name the correction pattern.
