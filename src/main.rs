mod models;
mod utils;

use anyhow::Result;
use async_recursion::async_recursion;
use clap::Parser;
use cmd_lib::{run_cmd, spawn_with_output};
use fancy_regex::Regex;
use serde::Deserialize;
use std::{
    fs::{create_dir_all, File},
    process,
};
use tempdir::TempDir;

use utils::*;

fn get_solver(solvers: &serde_yaml::Value, query: &str) -> Result<serde_yaml::Value> {
    if let serde_yaml::Value::Mapping(s) = solvers.clone() {
        if s.contains_key(&serde_yaml::Value::String(query.to_string())) {
            return Ok(s[&serde_yaml::Value::String(query.to_string())].clone());
        } else {
            println!("No solver named {}", query);
            process::exit(2);
        }
    } else {
        println!("Error parsing solver config");
        process::exit(3);
    }
}

fn match_key(matcher: &models::Matcher, query: &str) -> Result<(bool, String)> {
    let re = Regex::new(&matcher.key)?;
    match re.captures(query)? {
        Some(res) => match res.name("actual_key") {
            Some(k) => Ok((true, k.as_str().to_string())),
            None => Ok((false, "".to_string())),
        },
        None => Ok((false, "".to_string())),
    }
}

#[async_recursion]
async fn resolve(
    solvers: &serde_yaml::Value,
    doc: &serde_yaml::Value,
    new_doc: &mut serde_yaml::Value,
    matcher: &models::Matcher,
    current_level: i8,
) -> Result<()> {
    for (k, v) in doc.as_mapping().unwrap() {
        match v {
            serde_yaml::Value::Mapping(m) => {
                let mk = match_key(matcher, k.as_str().unwrap())?;
                if current_level >= matcher.level.min && current_level <= matcher.level.max && mk.0
                {
                    if let serde_yaml::Value::String(s) = v["solver"].clone() {
                        let solver = get_solver(solvers, &s)?;
                        let http_path = solver["location"]["file"].as_str().unwrap_or_default();
                        let arg_str = v["args"]
                            .as_sequence()
                            .unwrap()
                            .into_iter()
                            .map(|val| val.as_str().unwrap_or_default())
                            .collect::<Vec<&str>>();
                        new_doc[mk.1] = match solver["type"].as_str().unwrap() {
                            "bash" => {
                                let dir = TempDir::new("ymlex")?;
                                let script_path = dir.path().join("script.sh");
                                get_file_over_http(
                                    http_path,
                                    script_path.to_str().unwrap_or_default(),
                                )
                                .await?;
                                run_cmd!(
                                    chmod +x $script_path
                                )?;
                                serde_yaml::Value::String(
                                    spawn_with_output!($script_path $[arg_str] 2>&1)?
                                        .wait_with_output()?,
                                )
                            }
                            "python" => serde_yaml::Value::String("python".to_string()),
                            "binary" => serde_yaml::Value::String("binary".to_string()),
                            _ => serde_yaml::Value::String("".to_string()),
                        }
                    }
                } else {
                    new_doc[k] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    resolve(
                        solvers,
                        &doc[k],
                        &mut new_doc[k],
                        matcher,
                        current_level + 1,
                    )
                    .await?;
                }
            }
            _ => new_doc[k] = v.clone(),
        }
    }
    Ok(())
}

fn config_dir() -> String {
    format!(
        "{}/.config/ymlex",
        home::home_dir().unwrap_or_default().display()
    )
}

async fn setup_config() -> Result<()> {
    create_dir_all(config_dir()).unwrap();
    utils::get_file_over_http(
        "https://raw.githubusercontent.com/tidunguyen/ymlex/main/configs/default.ymlex.yml",
        &format!("{}/default.ymlex.yml", config_dir()),
    )
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = models::Args::parse();
    setup_config().await?;
    let default_config: serde_yaml::Value =
        serde_yaml::from_reader(File::open(format!("{}/default.ymlex.yml", config_dir()))?)?;
    let mut config: serde_yaml::Value = serde_yaml::Value::Null;
    let config_path = format!("{}/overlay.ymlex.yml", config_dir());
    if std::path::Path::new(&config_path).exists() {
        config =
            serde_yaml::from_reader(File::open(config_path)?)?;
    }

    overlaying_config(&default_config, &mut config)?;

    let validation = validate_config(serde_yaml::from_value::<serde_json::Value>(config.clone())?)?;

    if !validation.is_valid() {
        println!("Config does not follow schema. Please refer to schema.");
        println!("{:#?}", validation.errors);
        process::exit(1);
    }

    let mut matcher: models::Matcher = serde_yaml::from_value(config["matcher"].clone())?;
    if matcher.level.max == -1 {
        matcher.level.max = std::i8::MAX;
    }

    match args.input.as_str() {
        "stdin" => {
            let stdin = std::io::stdin();
            let stdin = stdin.lock();
            for doc_deserializer in serde_yaml::Deserializer::from_reader(stdin) {
                let doc = serde_yaml::Value::deserialize(doc_deserializer)?;
                let mut new_doc = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                resolve(&config["solvers"], &doc, &mut new_doc, &matcher, 0).await?;
                println!("{}", serde_yaml::to_string(&new_doc)?);
            }
        }
        _ => println!("{}", "Only support stdin for now"),
    }
    Ok(())
}
