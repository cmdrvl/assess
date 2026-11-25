use serde_json::{Value, json};

use crate::{
    ASSESS_SCHEMA_JSON, Execution, OPERATOR_JSON, POLICY_SCHEMA_JSON,
    cli::{AssessExit, DoctorInvocation, DoctorInvocationCommand},
    paths, witness,
};

/// Local read-only discipline label; kept under `read_only_contract` (the
/// `contract` key is reserved by cmdrvl.doctor.v1 for the operator.v1 document).
const CONTRACT: &str = "cmdrvl.read_only_doctor.v1";
const DOCTOR_SCHEMA_VERSION: &str = "cmdrvl.doctor.v1";
const OPERATOR_SCHEMA: &str = "operator.v1";

const CAPABILITIES_TEXT: &str = "assess capabilities\nread_only=true\nfixers=0\nagent_entrypoints=assess doctor triage --json,assess capabilities --json,assess robot-docs guide";

pub fn execute(invocation: DoctorInvocation) -> Execution {
    match invocation.command {
        DoctorInvocationCommand::Health if invocation.json => json_execution(health_payload()),
        DoctorInvocationCommand::Health => {
            let payload = health_payload();
            Execution::new(exit_for(&payload), health_summary(&payload))
        }
        // Top-level `capabilities` is the operator.v1 document: exactly the bytes
        // `--describe` prints, so the two surfaces cannot drift.
        DoctorInvocationCommand::Capabilities if invocation.json => {
            Execution::new(AssessExit::Proceed, OPERATOR_JSON)
        }
        DoctorInvocationCommand::Capabilities => {
            Execution::new(AssessExit::Proceed, CAPABILITIES_TEXT)
        }
        DoctorInvocationCommand::DoctorCapabilities if invocation.json => {
            json_execution(capabilities_payload())
        }
        DoctorInvocationCommand::DoctorCapabilities => {
            Execution::new(AssessExit::Proceed, CAPABILITIES_TEXT)
        }
        DoctorInvocationCommand::RobotDocs => {
            Execution::new(AssessExit::Proceed, robot_docs_text())
        }
        DoctorInvocationCommand::RobotTriage => json_execution(triage_payload()),
    }
}

/// Doctor exit code: 0 healthy, 1 unhealthy (never 2; 2 is usage errors only).
fn exit_for(payload: &Value) -> AssessExit {
    if payload
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        AssessExit::Proceed
    } else {
        AssessExit::Unhealthy
    }
}

fn json_execution(payload: Value) -> Execution {
    Execution::new(exit_for(&payload), json_string(&payload))
}

/// Turn a legacy-shaped payload into a cmdrvl.doctor.v1 report: add the
/// required envelope fields, drop the old per-tool `schema` id, and move the
/// old `contract` string out of the way. Extra read-only detail stays as
/// additional keys (the schema allows them).
fn into_doctor_report(mut payload: Value, mode: &str, findings: Vec<Value>) -> Value {
    let healthy = !findings
        .iter()
        .any(|f| f.get("severity").and_then(Value::as_str) == Some("error"));
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("schema");
        obj.remove("contract");
        obj.insert("read_only_contract".into(), json!(CONTRACT));
        obj.insert("schema_version".into(), json!(DOCTOR_SCHEMA_VERSION));
        obj.insert("mode".into(), json!(mode));
        obj.insert("tool_version".into(), json!(env!("CARGO_PKG_VERSION")));
        obj.insert("healthy".into(), json!(healthy));
        obj.insert("ok".into(), json!(healthy));
        obj.insert("findings".into(), Value::Array(findings));
    }
    payload
}

/// Like [`into_doctor_report`] but keeps an already-inserted `contract` object
/// (capabilities mode embeds the operator.v1 document there).
fn into_doctor_report_keep_contract(mut payload: Value, mode: &str, findings: Vec<Value>) -> Value {
    let contract = payload
        .as_object_mut()
        .and_then(|obj| obj.remove("contract"));
    let mut report = into_doctor_report(payload, mode, findings);
    if let (Some(obj), Some(contract)) = (report.as_object_mut(), contract) {
        obj.insert("contract".into(), contract);
    }
    report
}

/// One error finding per failed health check.
fn findings_from_checks(checks: &[Value]) -> Vec<Value> {
    checks
        .iter()
        .filter(|check| !check.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .map(|check| {
            json!({
                "id": check.get("id").cloned().unwrap_or(Value::Null),
                "severity": "error",
                "message": check.get("message").cloned().unwrap_or(Value::Null),
                "fix": "Reinstall assess; the compiled manifest or schemas are inconsistent."
            })
        })
        .collect()
}

fn json_string(payload: &Value) -> String {
    match serde_json::to_string(payload) {
        Ok(encoded) => encoded,
        Err(error) => format!(
            "{{\"schema_version\":\"{DOCTOR_SCHEMA_VERSION}\",\"tool\":\"assess\",\"healthy\":false,\"error\":\"json encode failed: {error}\"}}"
        ),
    }
}

fn health_payload() -> Value {
    let checks = health_checks();
    let failed = checks
        .iter()
        .filter(|check| !check.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let total = checks.len();
    let passed = total.saturating_sub(failed);

    let findings = findings_from_checks(&checks);
    let payload = json!({
        "schema": "legacy",
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "read_only": true,
        "summary": {
            "checks_total": total,
            "checks_passed": passed,
            "checks_failed": failed
        },
        "checks": checks,
        "observed_paths": observed_paths(),
        "config_footprint": paths::config_footprint(),
        "side_effects": side_effects(),
        "fixers": []
    });
    into_doctor_report(payload, "health", findings)
}

fn capabilities_payload() -> Value {
    let checks = health_checks();
    let findings = findings_from_checks(&checks);
    let mut payload = capabilities_detail();
    if let Some(obj) = payload.as_object_mut() {
        // The embedded contract is the operator.v1 document, identical to
        // `assess capabilities --json` / `assess --describe`.
        obj.insert(
            "contract".into(),
            serde_json::from_str(OPERATOR_JSON).unwrap_or(Value::Null),
        );
    }
    into_doctor_report_keep_contract(payload, "capabilities", findings)
}

fn capabilities_detail() -> Value {
    json!({
        "schema": "legacy",
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "read_only": true,
        "agent_entrypoints": [
            {
                "name": "robot-triage",
                "usage": "assess doctor triage --json",
                "output_schema": DOCTOR_SCHEMA_VERSION,
                "description": "Single-call read-only health, command, and next-step report"
            },
            {
                "name": "capabilities",
                "usage": "assess capabilities --json",
                "output_schema": OPERATOR_SCHEMA,
                "description": "Machine-readable CLI, refusal, side-effect, and schema contract"
            },
            {
                "name": "robot-docs",
                "usage": "assess robot-docs guide",
                "output_schema": "text/plain",
                "description": "Paste-ready agent guide for decision, witness, and doctor workflows"
            }
        ],
        "commands": [
            {
                "name": "run",
                "usage": "assess <ARTIFACT>... --policy <PATH> --json",
                "output_schema": "assess.v0",
                "description": "Classify a complete evidence bundle against an explicit policy file"
            },
            {
                "name": "run-by-policy-id",
                "usage": "assess <ARTIFACT>... --policy-id <ID> --json",
                "output_schema": "assess.v0",
                "description": "Classify a bundle against a policy resolved from ASSESS_POLICY_PATH, builtins, or ~/.cmdrvl/config/assess/policies"
            },
            {
                "name": "summary",
                "usage": "assess <ARTIFACT>... --policy-id <ID> --render summary",
                "output_schema": "text/plain",
                "description": "Emit one-line operator summary for shell pipelines"
            },
            {
                "name": "summary-tsv",
                "usage": "assess <ARTIFACT>... --policy-id <ID> --render summary-tsv",
                "output_schema": "text/tab-separated-values",
                "description": "Emit stable header plus row TSV summary"
            },
            {
                "name": "describe",
                "usage": "assess --describe",
                "output_schema": "operator.v0",
                "description": "Emit embedded operator manifest before normal validation"
            },
            {
                "name": "schema",
                "usage": "assess --schema",
                "output_schema": "json-schema",
                "description": "Emit assess.v0 JSON Schema before normal validation"
            },
            {
                "name": "version",
                "usage": "assess --version",
                "output_schema": "text/plain",
                "description": "Emit compiled semantic version"
            },
            {
                "name": "doctor-health",
                "usage": "assess doctor health --json",
                "output_schema": DOCTOR_SCHEMA_VERSION,
                "description": "Report compiled manifest, embedded schema, and read-only contract health"
            },
            {
                "name": "doctor-capabilities",
                "usage": "assess doctor capabilities --json",
                "output_schema": DOCTOR_SCHEMA_VERSION,
                "description": "Describe doctor commands, exit codes, side-effect boundaries, and disabled fixers"
            },
            {
                "name": "doctor-robot-docs",
                "usage": "assess doctor robot-docs",
                "output_schema": "text/plain",
                "description": "Emit concise machine-oriented usage notes"
            },
            {
                "name": "doctor-triage",
                "usage": "assess doctor triage --json",
                "output_schema": DOCTOR_SCHEMA_VERSION,
                "description": "Canonical triage report; `assess doctor --robot-triage` and `assess --robot-triage` are deprecated aliases"
            },
            {
                "name": "doctor-robot-triage",
                "usage": "assess doctor --robot-triage",
                "output_schema": DOCTOR_SCHEMA_VERSION,
                "description": "Emit a compact triage report for automation"
            },
            {
                "name": "witness-last",
                "usage": "assess witness last --json",
                "output_schema": "assess.witness.record.v1 | null",
                "description": "Read the most recent local witness receipt"
            },
            {
                "name": "witness-query",
                "usage": "assess witness query policy=<ID> --json",
                "output_schema": "array<assess.witness.record.v1>",
                "description": "Filter local witness receipts by key=value filters"
            },
            {
                "name": "witness-count",
                "usage": "assess witness count policy=<ID> --json",
                "output_schema": "count",
                "description": "Count local witness receipts matching key=value filters"
            }
        ],
        "exit_codes": {
            "0": "PROCEED, PROCEED_WITH_RISK, metadata emitted successfully, or read-only introspection succeeded",
            "1": "ESCALATE (doctor: unhealthy)",
            "2": "structured refusal, CLI usage error, or witness query I/O error (never a policy verdict)",
            "3": "BLOCK (valid policy verdict, distinct from refusal)"
        },
        "refusal_codes": [
            {
                "code": "E_BAD_POLICY",
                "next_command": "assess <ARTIFACT>... --policy <PATH> --json"
            },
            {
                "code": "E_AMBIGUOUS_POLICY",
                "next_command": "assess <ARTIFACT>... --policy <PATH> --json"
            },
            {
                "code": "E_UNKNOWN_POLICY",
                "next_command": "assess <ARTIFACT>... --policy <PATH> --json"
            },
            {
                "code": "E_BAD_ARTIFACT",
                "next_command": "assess <ARTIFACT>... --policy <PATH> --json"
            },
            {
                "code": "E_DUPLICATE_TOOL",
                "next_command": "assess <ONE_ARTIFACT_PER_TOOL>... --policy <PATH> --json"
            },
            {
                "code": "E_INCOMPLETE_BASIS",
                "next_command": "assess <ALL_REQUIRED_ARTIFACTS>... --policy <PATH> --json"
            },
            {
                "code": "E_MISSING_RULE",
                "next_command": "assess <ARTIFACT>... --policy <PATH_WITH_DEFAULT_RULE> --json"
            }
        ],
        "schemas": {
            "decision": "assess.v0",
            "policy": "policy.v0",
            "doctor_health": DOCTOR_SCHEMA_VERSION,
            "doctor_capabilities": DOCTOR_SCHEMA_VERSION,
            "doctor_triage": DOCTOR_SCHEMA_VERSION
        },
        "composition": {
            "family": {
                "name": "cmdrvl-spine",
                "siblings": ["shape", "rvl", "verify", "benchmark", "lock", "pack"],
                "capabilities": {
                    "shape": "shape capabilities --json",
                    "rvl": "rvl capabilities --json",
                    "verify": "verify capabilities --json",
                    "benchmark": "benchmark capabilities --json",
                    "assess": "assess capabilities --json",
                    "pack": "pack capabilities --json"
                }
            },
            "role": "decision_classifier",
            "position": "after_evidence_before_pack",
            "accepts": [
                {
                    "kind": "evidence_bundle",
                    "description": "One or more tool artifacts from shape, rvl, verify, benchmark, or other declared basis tools."
                },
                {
                    "kind": "policy",
                    "description": "A policy.v0 YAML file or policy id resolved from the assess policy path."
                }
            ],
            "produces": [
                {
                    "kind": "decision_artifact",
                    "schema": "assess.v0",
                    "command": "assess <ARTIFACT>... --policy <policy.yaml> --json"
                }
            ],
            "canonical_chains": [
                {
                    "name": "evidence_to_decision_to_pack",
                    "commands": [
                        "shape <old.csv> <new.csv> --key <column> --json > shape.json",
                        "rvl <old.csv> <new.csv> --key <column> --json > rvl.json",
                        "verify run <constraints.verify.json> --bind input=<dataset.csv> --json > verify.json",
                        "benchmark <CANDIDATE> --assertions <FILE> --key <COLUMN> --json > benchmark.json",
                        "assess shape.json rvl.json verify.json benchmark.json --policy <policy.yaml> --json > decision.json",
                        "pack seal shape.json rvl.json verify.json benchmark.json decision.json --output evidence/"
                    ],
                    "upstream_tools": ["shape", "rvl", "verify", "benchmark"],
                    "downstream_tools": ["pack"]
                }
            ],
            "agent_rules": [
                "Use assess only after upstream tools have produced the evidence artifacts named by policy.",
                "Do not ask assess to score, compare, validate constraints, or produce facts; it classifies supplied facts.",
                "Seal assess decisions with pack when the result needs to be portable evidence."
            ]
        },
        "config_footprint": paths::config_footprint(),
        "side_effects": side_effects(),
        "fixers": []
    })
}

fn triage_payload() -> Value {
    let health = health_payload();
    let ok = health.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let checks = health
        .get("checks")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let findings = findings_from_checks(checks.as_array().map(Vec::as_slice).unwrap_or_default());
    let payload = json!({
        "schema": "legacy",
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "score": if ok { 100 } else { 0 },
        "read_only": true,
        "checks": checks,
        "config_footprint": paths::config_footprint(),
        "side_effects": side_effects(),
        "fixers": [],
        "next_actions": [
            "Use assess capabilities --json for the full machine-readable CLI contract.",
            "Use assess robot-docs guide for the compact agent guide.",
            "Use assess --schema for the assess.v0 decision output schema.",
            "Do not expect assess doctor to read artifacts, load policies, evaluate rules, query or append witness records, or emit portable evidence."
        ]
    });
    into_doctor_report(payload, "triage", findings)
}

fn health_checks() -> Vec<Value> {
    let manifest = serde_json::from_str::<Value>(OPERATOR_JSON).ok();
    let manifest_version = manifest
        .as_ref()
        .and_then(|value| value.get("tool_version"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let output_schema = manifest
        .as_ref()
        .and_then(|value| value.get("invocation"))
        .and_then(|value| value.get("output_schema"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let doctor_declared = manifest
        .as_ref()
        .and_then(|value| value.get("commands"))
        .and_then(Value::as_array)
        .map(|subcommands| {
            subcommands
                .iter()
                .any(|entry| entry.get("name").and_then(Value::as_str) == Some("doctor"))
        })
        .unwrap_or(false);

    vec![
        check(
            "operator_manifest_embedded",
            manifest.is_some(),
            "compiled operator.json parses as JSON",
        ),
        check(
            "operator_manifest_version",
            manifest_version == env!("CARGO_PKG_VERSION"),
            "operator.json version matches the compiled crate version",
        ),
        check(
            "operator_output_schema",
            output_schema == "assess.v0",
            "operator.json declares the assess.v0 output contract",
        ),
        check(
            "assess_schema_embedded",
            serde_json::from_str::<Value>(ASSESS_SCHEMA_JSON).is_ok(),
            "compiled assess.v0 schema parses as JSON",
        ),
        check(
            "policy_schema_embedded",
            serde_json::from_str::<Value>(POLICY_SCHEMA_JSON).is_ok(),
            "compiled policy.v0 schema parses as JSON",
        ),
        check(
            "doctor_manifest_entry",
            doctor_declared,
            "operator.json declares the read-only doctor subcommand",
        ),
        check(
            "fix_mode_disabled",
            true,
            "doctor --fix is intentionally absent from the CLI surface",
        ),
        check(
            "witness_ledger_unopened",
            true,
            "doctor does not query or append the local witness ledger",
        ),
        check(
            "config_footprint_declared",
            true,
            "implicit policy and witness paths are rooted under ~/.cmdrvl",
        ),
        check(
            "decision_pipeline_unentered",
            true,
            "doctor does not load policies, construct bundles, or evaluate rules",
        ),
        check(
            "network_disabled",
            true,
            "doctor performs no DNS, HTTP, TLS, or other network probes",
        ),
    ]
}

fn check(id: &str, ok: bool, message: &str) -> Value {
    json!({
        "id": id,
        "ok": ok,
        "severity": if ok { "info" } else { "error" },
        "message": message
    })
}

fn observed_paths() -> Value {
    json!({
        "operator_manifest": "embedded:operator.json",
        "assess_schema": "embedded:schemas/assess.v0.schema.json",
        "policy_schema": "embedded:schemas/policy.v0.schema.json",
        "policy_dir": paths::canonical_policy_dir().display().to_string(),
        "witness_ledger": witness::ledger::witness_ledger_path().display().to_string()
    })
}

fn side_effects() -> Value {
    json!({
        "reads_stdin": false,
        "reads_artifact_files": false,
        "reads_policy_files": false,
        "validates_policy": false,
        "constructs_bundle": false,
        "evaluates_rules": false,
        "renders_decision": false,
        "queries_witness_ledger": false,
        "opens_witness_ledger": false,
        "appends_witness_ledger": false,
        "creates_witness_directory": false,
        "writes_witness_ledger": false,
        "writes_migration_logs": false,
        "writes_deprecation_notices": false,
        "writes_doctor_artifacts": false,
        "uses_network": false,
        "changes_cwd": false,
        "rewrites_operator_manifest": false
    })
}

fn health_summary(payload: &Value) -> String {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let passed = payload
        .get("summary")
        .and_then(|summary| summary.get("checks_passed"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total = payload
        .get("summary")
        .and_then(|summary| summary.get("checks_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = if ok { "ok" } else { "unhealthy" };
    format!("assess doctor health: {status}\nread_only=true\nfixers=0\nchecks={passed}/{total}")
}

fn robot_docs_text() -> &'static str {
    "assess robot-docs guide\n\
contract: cmdrvl.read_only_doctor.v1\n\
agent_entrypoints:\n\
  assess doctor triage --json  (deprecated alias: assess --robot-triage)\n\
  assess capabilities --json\n\
  assess robot-docs guide\n\
decision:\n\
  assess <ARTIFACT>... --policy <PATH> --json\n\
  assess <ARTIFACT>... --policy-id <ID> --render summary\n\
witness:\n\
  assess witness last --json\n\
  assess witness query policy=<ID> --json\n\
doctor_compat:\n\
  assess doctor health --json\n\
  assess doctor capabilities --json\n\
  assess doctor triage --json\n\
  assess doctor --robot-triage  (deprecated alias)\n\
doctor_exit: 0 healthy, 1 unhealthy, 2 usage error\n\
composition:\n\
  assess shape.json rvl.json verify.json benchmark.json --policy <policy.yaml> --json > decision.json\n\
  pack seal shape.json rvl.json verify.json benchmark.json decision.json --output evidence/\n\
exit_codes:\n\
  0 = PROCEED, PROCEED_WITH_RISK, or read-only metadata success\n\
  1 = ESCALATE (doctor: unhealthy)\n\
  2 = refusal, CLI usage error, or witness query I/O error (never a policy verdict)\n\
  3 = BLOCK (valid policy verdict, distinct from refusal)\n\
read_only:\n\
  - does not read stdin, artifacts, policies, schemas from disk, or witness ledgers\n\
  - does not construct bundles, evaluate policy rules, append witness records, create directories, or contact providers\n\
  - implicit policy fallback is ~/.cmdrvl/config/assess/policies; implicit witness fallback is ~/.cmdrvl/state/witness/witness.jsonl\n\
fix_mode:\n\
  - no --fix surface is implemented in this release\n\
next_steps:\n\
  - use assess capabilities --json for the full machine-readable command/refusal contract\n\
  - use assess --describe for the full operator manifest\n\
  - use assess --schema for the decision artifact schema"
}

#[cfg(test)]
mod tests {
    use super::*;
    use spine_rules::doctor::{DoctorMode, check_doctor_run};

    fn failing_checks() -> Vec<Value> {
        vec![
            check("operator_manifest_embedded", true, "parses"),
            check("assess_schema_embedded", false, "schema does not parse"),
        ]
    }

    #[test]
    fn failed_check_makes_an_unhealthy_report_that_exits_one() {
        let checks = failing_checks();
        let findings = findings_from_checks(&checks);
        let report = into_doctor_report(json!({"tool": "assess"}), "health", findings);

        assert_eq!(report["healthy"], false);
        assert_eq!(report["findings"][0]["id"], "assess_schema_embedded");
        assert_eq!(report["findings"][0]["severity"], "error");
        assert_eq!(exit_for(&report).code(), 1);

        // Conforms to cmdrvl.doctor.v1 only with exit 1 (unhealthy != usage error 2).
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(check_doctor_run(DoctorMode::Health, &bytes, Some(1)).is_empty());
        let wrong = check_doctor_run(DoctorMode::Health, &bytes, Some(2)).join("\n");
        assert!(wrong.contains("requires 1"), "{wrong}");
        let wrong = check_doctor_run(DoctorMode::Health, &bytes, Some(0)).join("\n");
        assert!(wrong.contains("requires 1"), "{wrong}");
    }

    #[test]
    fn healthy_report_exits_zero_and_validates_in_all_modes() {
        for (mode, label) in [
            (DoctorMode::Health, "health"),
            (DoctorMode::Triage, "triage"),
        ] {
            let payload = match label {
                "health" => health_payload(),
                _ => triage_payload(),
            };
            assert_eq!(exit_for(&payload).code(), 0);
            let bytes = serde_json::to_vec(&payload).unwrap();
            let v = check_doctor_run(mode, &bytes, Some(0));
            assert!(v.is_empty(), "{label}: {v:?}");
        }
        let caps = capabilities_payload();
        let bytes = serde_json::to_vec(&caps).unwrap();
        let v = check_doctor_run(DoctorMode::Capabilities, &bytes, Some(0));
        assert!(v.is_empty(), "capabilities: {v:?}");
        // The embedded contract is exactly the operator document.
        assert_eq!(
            caps["contract"],
            serde_json::from_str::<Value>(OPERATOR_JSON).unwrap()
        );
    }

    #[test]
    fn unhealthy_triage_carries_next_actions() {
        let findings = findings_from_checks(&failing_checks());
        let report = into_doctor_report(
            json!({"tool": "assess", "next_actions": ["reinstall assess"]}),
            "triage",
            findings,
        );
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(check_doctor_run(DoctorMode::Triage, &bytes, Some(1)).is_empty());
    }
}
