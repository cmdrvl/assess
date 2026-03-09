#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessExit {
    Proceed,
    Attention,
    Stop,
}

impl AssessExit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Proceed => 0,
            Self::Attention => 1,
            Self::Stop => 2,
        }
    }
}
