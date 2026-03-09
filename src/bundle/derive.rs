pub fn canonical_tool(explicit_tool: Option<&str>, version: &str) -> Option<String> {
    match explicit_tool {
        Some(tool) => normalize_explicit_tool(tool),
        None => derive_from_version(version),
    }
}

fn normalize_explicit_tool(tool: &str) -> Option<String> {
    let trimmed = tool.trim();
    is_valid_tool_name(trimmed).then(|| trimmed.to_owned())
}

fn derive_from_version(version: &str) -> Option<String> {
    let (tool, suffix) = version.rsplit_once(".v")?;
    (!tool.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) && is_valid_tool_name(tool))
        .then(|| tool.to_owned())
}

fn is_valid_tool_name(tool: &str) -> bool {
    !tool.is_empty()
        && tool
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '-'))
}
