use serde_json::json;

use assess::refusal::{RefusalCode, RefusalEnvelope};

#[test]
fn refusal_code_set_matches_plan_count() {
    let labels: Vec<&str> = RefusalCode::ALL
        .into_iter()
        .map(RefusalCode::as_str)
        .collect();
    assert_eq!(labels.len(), 7);
    assert!(labels.contains(&"E_BAD_POLICY"));
    assert!(labels.contains(&"E_MISSING_RULE"));
}

#[test]
fn refusal_codes_serialize_to_protocol_labels() {
    for code in RefusalCode::ALL {
        let encoded = serde_json::to_string(&code).expect("refusal code should serialize");
        assert_eq!(encoded, format!("\"{}\"", code.as_str()));
    }
}

#[test]
fn refusal_codes_all_expose_next_command_guidance() {
    for code in RefusalCode::ALL {
        assert_eq!(code.next_command(), "assess <ARTIFACT>... --policy <PATH>");
    }
}

#[test]
fn refusal_envelope_uses_assess_protocol_defaults() {
    let envelope = RefusalEnvelope::new(RefusalCode::BadPolicy, "policy default rule must be last");

    assert_eq!(envelope.tool, "assess");
    assert_eq!(envelope.version, "assess.v0");
    assert_eq!(envelope.decision_band, None);
    assert_eq!(envelope.refusal.code, RefusalCode::BadPolicy);
    assert_eq!(envelope.refusal.detail, json!({}));
    assert_eq!(
        envelope.refusal.next_command,
        "assess <ARTIFACT>... --policy <PATH>"
    );
}

#[test]
fn refusal_envelope_supports_detail_and_command_override() {
    let envelope = RefusalEnvelope::new(
        RefusalCode::UnknownPolicy,
        "policy id loan_tape.monthly.v1 was not found",
    )
    .with_detail(json!({
        "policy_id": "loan_tape.monthly.v1",
        "search_path": ["/tmp/policies"],
    }))
    .with_next_command(
        "assess artifacts/*.json --policy fixtures/policies/loan_tape_monthly_v1.yaml",
    );

    assert_eq!(
        envelope.refusal.detail,
        json!({
            "policy_id": "loan_tape.monthly.v1",
            "search_path": ["/tmp/policies"],
        })
    );
    assert_eq!(
        envelope.refusal.next_command,
        "assess artifacts/*.json --policy fixtures/policies/loan_tape_monthly_v1.yaml"
    );
}

#[test]
fn refusal_envelope_json_roundtrip_preserves_c02_contract() {
    for code in RefusalCode::ALL {
        let envelope = RefusalEnvelope::new(code, format!("test message for {}", code.as_str()));
        let json_str = envelope.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["tool"], "assess");
        assert_eq!(parsed["version"], "assess.v0");
        assert!(parsed["decision_band"].is_null());
        assert_eq!(parsed["refusal"]["code"], code.as_str());
        assert!(parsed["refusal"]["message"].is_string());
        assert!(parsed["refusal"]["detail"].is_object());
        assert!(parsed["refusal"]["next_command"].is_string());
    }
}
