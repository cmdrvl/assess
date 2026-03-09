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
