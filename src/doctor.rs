use serde_json::{Value, json};

use crate::{
    ASSESS_SCHEMA_JSON, Execution, OPERATOR_JSON, POLICY_SCHEMA_JSON,
    cli::{AssessExit, DoctorInvocation, DoctorInvocationCommand},
    paths, witness,
};

const CONTRACT: &str = "cmdrvl.read_only_doctor.v1";
const HEALTH_SCHEMA: &str = "assess.doctor.health.v1";
const CAPABILITIES_SCHEMA: &str = "assess.doctor.capabilities.v1";
const TRIAGE_SCHEMA: &str = "assess.doctor.triage.v1";

pub fn execute(invocation: DoctorInvocation) -> Execution {
    match invocation.command {
        DoctorInvocationCommand::Health if invocation.json => json_execution(health_payload()),
        DoctorInvocationCommand::Health => {
            Execution::new(AssessExit::Proceed, health_summary(&health_payload()))
        }
        DoctorInvocationCommand::Capabilities if invocation.json => {
            json_execution(capabilities_payload())
        }
        DoctorInvocationCommand::Capabilities => Execution::new(
            AssessExit::Proceed,
            "assess capabilities\nread_only=true\nfixers=0\nagent_entrypoints=assess --robot-triage,assess capabilities --json,assess robot-docs guide",
        ),
        DoctorInvocationCommand::RobotDocs => {
            Execution::new(AssessExit::Proceed, robot_docs_text())
        }
        DoctorInvocationCommand::RobotTriage => json_execution(triage_payload()),
    }
}

fn json_execution(payload: Value) -> Execution {
    Execution::new(AssessExit::Proceed, json_string(&payload))
}

fn json_string(payload: &Value) -> String {
    match serde_json::to_string(payload) {
        Ok(encoded) => encoded,
        Err(error) => format!(
            "{{\"schema\":\"{HEALTH_SCHEMA}\",\"contract\":\"{CONTRACT}\",\"tool\":\"assess\",\"ok\":false,\"error\":\"json encode failed: {error}\"}}"
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

    json!({
        "schema": HEALTH_SCHEMA,
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "ok": failed == 0,
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
    })
}

fn capabilities_payload() -> Value {
    json!({
        "schema": CAPABILITIES_SCHEMA,
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "read_only": true,
        "agent_entrypoints": [
            {
                "name": "robot-triage",
                "usage": "assess --robot-triage",
                "output_schema": TRIAGE_SCHEMA,
                "description": "Single-call read-only health, command, and next-step report"
            },
            {
                "name": "capabilities",
                "usage": "assess capabilities --json",
                "output_schema": CAPABILITIES_SCHEMA,
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
                "output_schema": HEALTH_SCHEMA,
                "description": "Report compiled manifest, embedded schema, and read-only contract health"
            },
            {
                "name": "doctor-capabilities",
                "usage": "assess doctor capabilities --json",
                "output_schema": CAPABILITIES_SCHEMA,
                "description": "Describe doctor commands, exit codes, side-effect boundaries, and disabled fixers"
            },
            {
                "name": "doctor-robot-docs",
                "usage": "assess doctor robot-docs",
                "output_schema": "text/plain",
                "description": "Emit concise machine-oriented usage notes"
            },
            {
                "name": "doctor-robot-triage",
                "usage": "assess doctor --robot-triage",
                "output_schema": TRIAGE_SCHEMA,
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
            "0": "PROCEED, metadata emitted successfully, or read-only introspection succeeded",
            "1": "PROCEED_WITH_RISK or ESCALATE",
            "2": "BLOCK, structured refusal, CLI usage error, or witness I/O error"
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
            "doctor_health": HEALTH_SCHEMA,
            "doctor_capabilities": CAPABILITIES_SCHEMA,
            "doctor_triage": TRIAGE_SCHEMA
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

    json!({
        "schema": TRIAGE_SCHEMA,
        "contract": CONTRACT,
        "tool": "assess",
        "version": env!("CARGO_PKG_VERSION"),
        "ok": ok,
        "score": if ok { 100 } else { 0 },
        "read_only": true,
        "checks": checks,
        "config_footprint": paths::config_footprint(),
        "side_effects": side_effects(),
        "fixers": [],
        "recommended_next_steps": [
            "Use assess capabilities --json for the full machine-readable CLI contract.",
            "Use assess robot-docs guide for the compact agent guide.",
            "Use assess --schema for the assess.v0 decision output schema.",
            "Do not expect assess doctor to read artifacts, load policies, evaluate rules, query or append witness records, or emit portable evidence."
        ]
    })
}

fn health_checks() -> Vec<Value> {
    let manifest = serde_json::from_str::<Value>(OPERATOR_JSON).ok();
    let manifest_version = manifest
        .as_ref()
        .and_then(|value| value.get("version"))
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
        .and_then(|value| value.get("subcommands"))
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
  assess --robot-triage\n\
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
  assess doctor --robot-triage\n\
exit_codes:\n\
  0 = PROCEED or read-only metadata success\n\
  1 = PROCEED_WITH_RISK or ESCALATE\n\
  2 = BLOCK, refusal, CLI usage error, or witness I/O error\n\
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
