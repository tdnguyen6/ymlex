use clap::Parser;
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

#[derive(Parser, Debug)]
#[clap(name = "Yamlex")]
#[clap(author = "TiDu Nguyen <tidu.nguyen.2000@gmail.com>")]
#[clap(version = "0.0.1")]
pub struct Args {
    #[clap(default_value_t = String::from("stdin"))]
    pub input: String,
}
