pub fn canonical_tool(explicit_tool: Option<&str>, version: &str) -> Option<String> {
    explicit_tool.map(str::to_owned).or_else(|| {
        version
            .split_once(".v")
            .map(|(tool, _)| tool.to_owned())
            .filter(|tool| !tool.is_empty())
    })
}
