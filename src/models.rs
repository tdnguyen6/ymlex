use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Matcher {
    pub key: String,
    pub level: MatcherLevel,
}

#[derive(Debug, Deserialize)]
pub struct MatcherLevel {
    pub min: i8,
    pub max: i8,
}
