use std::path::Path;

use serde_json::json;

use super::{ledger, record::WitnessRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessQueryMode {
    Query,
    Last,
    Count,
}

pub fn supported_modes() -> [WitnessQueryMode; 3] {
    [
        WitnessQueryMode::Query,
        WitnessQueryMode::Last,
        WitnessQueryMode::Count,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessCommandOutput {
    pub exit_code: u8,
    pub stdout: String,
}

impl WitnessCommandOutput {
    fn success(stdout: String) -> Self {
        Self {
            exit_code: 0,
            stdout,
        }
    }

    fn no_match(stdout: String) -> Self {
        Self {
            exit_code: 1,
            stdout,
        }
    }
}

pub fn render_query(filters: &[String], json_output: bool) -> Result<WitnessCommandOutput, String> {
    render_query_from_path(&ledger::witness_ledger_path(), filters, json_output)
}

pub fn render_query_from_path(
    path: &Path,
    filters: &[String],
    json_output: bool,
) -> Result<WitnessCommandOutput, String> {
    let records = query_from_path(path, filters)?;

    if records.is_empty() {
        return Ok(if json_output {
            WitnessCommandOutput::no_match("[]".to_owned())
        } else {
            WitnessCommandOutput::no_match(
                "assess: witness ledger has no matching records".to_owned(),
            )
        });
    }

    Ok(if json_output {
        WitnessCommandOutput::success(
            serde_json::to_string(&records).map_err(|error| {
                format!("assess: witness: failed to encode query result: {error}")
            })?,
        )
    } else {
        WitnessCommandOutput::success(
            records
                .iter()
                .map(format_record_human)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })
}

pub fn render_last(json_output: bool) -> Result<WitnessCommandOutput, String> {
    render_last_from_path(&ledger::witness_ledger_path(), json_output)
}

pub fn render_last_from_path(
    path: &Path,
    json_output: bool,
) -> Result<WitnessCommandOutput, String> {
    let Some(record) = last_from_path(path)? else {
        return Ok(if json_output {
            WitnessCommandOutput::no_match("null".to_owned())
        } else {
            WitnessCommandOutput::no_match("assess: witness ledger is empty".to_owned())
        });
    };

    Ok(if json_output {
        WitnessCommandOutput::success(
            serde_json::to_string(&record).map_err(|error| {
                format!("assess: witness: failed to encode latest record: {error}")
            })?,
        )
    } else {
        WitnessCommandOutput::success(format_record_human(&record))
    })
}

pub fn render_count(filters: &[String], json_output: bool) -> Result<WitnessCommandOutput, String> {
    render_count_from_path(&ledger::witness_ledger_path(), filters, json_output)
}

pub fn render_count_from_path(
    path: &Path,
    filters: &[String],
    json_output: bool,
) -> Result<WitnessCommandOutput, String> {
    let count = count_from_path(path, filters)?;
    if count == 0 {
        return Ok(if json_output {
            WitnessCommandOutput::no_match(json!({ "count": 0 }).to_string())
        } else {
            WitnessCommandOutput::no_match("0".to_owned())
        });
    }

    Ok(if json_output {
        WitnessCommandOutput::success(json!({ "count": count }).to_string())
    } else {
        WitnessCommandOutput::success(count.to_string())
    })
}

pub fn query_from_path(path: &Path, filters: &[String]) -> Result<Vec<WitnessRecord>, String> {
    let records = ledger::load_from_path(path)?;
    let mut matched = Vec::new();
    for record in records {
        if matches_filters(&record, filters)? {
            matched.push(record);
        }
    }
    Ok(matched)
}

pub fn last_from_path(path: &Path) -> Result<Option<WitnessRecord>, String> {
    Ok(ledger::load_from_path(path)?.into_iter().last())
}

pub fn count_from_path(path: &Path, filters: &[String]) -> Result<usize, String> {
    Ok(query_from_path(path, filters)?.len())
}

fn matches_filters(record: &WitnessRecord, filters: &[String]) -> Result<bool, String> {
    for filter in filters {
        let (key, value) = filter.split_once('=').ok_or_else(|| {
            format!("assess: witness: invalid filter `{filter}`; expected key=value")
        })?;

        let matched = match key {
            "policy" => record.policy_id.as_deref() == Some(value),
            "decision_band" => record.decision_band.as_deref() == Some(value),
            "input" => record.inputs.iter().any(|input| input == value),
            "tool" => record.tool == value,
            other => {
                return Err(format!(
                    "assess: witness: unsupported filter key `{other}`; use policy, decision_band, input, or tool"
                ));
            }
        };

        if !matched {
            return Ok(false);
        }
    }

    Ok(true)
}

fn format_record_human(record: &WitnessRecord) -> String {
    let policy = record.policy_id.as_deref().unwrap_or("-");
    let decision_band = record.decision_band.as_deref().unwrap_or("-");
    format!(
        "{} {} policy={} inputs={} duration_ms={}",
        record.ts,
        decision_band,
        policy,
        record.inputs.len(),
        record.duration_ms
    )
}
