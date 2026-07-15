#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Version {
    pub scope: Option<String>,
    pub major: u64, // year
    pub minor: u64, // month
    pub micro: u64, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Op {
    #[default]
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
    Wildcard,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Requirement {
    pub op: Op,
    pub scope: Option<String>,
    pub major: Option<u64>, // year
    pub minor: Option<u64>, // month
    pub micro: Option<u64>, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}
