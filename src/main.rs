mod models;

use std::{fs::File, process};

use anyhow::Result;
use cmd_lib::{run_cmd, spawn_with_output};
use fancy_regex::Regex;
use serde::Deserialize;
use valico::json_schema::{self, ValidationState};

fn validate_config(config: serde_json::Value) -> Result<ValidationState> {
    let json_schema: serde_json::Value =
        serde_json::from_reader(File::open("configs/ymlex.schema.json").unwrap()).unwrap();

    let mut scope = json_schema::Scope::new();
    let schema = scope
        .compile_and_return(json_schema.clone(), false)
        .unwrap();

    Ok(schema.validate(&config))
}

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

fn resolve(
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
                                run_cmd!(
                                    curl -LO $http_path;
                                    chmod +x script.sh
                                )?;
                                serde_yaml::Value::String(
                                    spawn_with_output!(./script.sh $[arg_str] 2>&1)?
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
                    )?;
                }
            }
            _ => new_doc[k] = v.clone(),
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let config: serde_yaml::Value = serde_yaml::from_reader(File::open("configs/main.ymlex.yml")?)?;
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

    for doc_deserializer in
        serde_yaml::Deserializer::from_reader(File::open("examples/example.yml")?)
    {
        let doc = serde_yaml::Value::deserialize(doc_deserializer)?;
        let mut new_doc = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        resolve(&config["solvers"], &doc, &mut new_doc, &matcher, 0)?;
        println!("{}", serde_yaml::to_string(&new_doc)?);
    }
    Ok(())
}
