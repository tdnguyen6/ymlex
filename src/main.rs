mod models;

use std::{fs::File, process};

use anyhow::Result;
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

fn resolve(
    doc: &serde_yaml::Value,
    matcher: &models::Matcher,
    current_level: i8,
) -> Result<()> {
    println!("{:#?}", current_level);
    if current_level >= matcher.level.min && current_level <= matcher.level.max {
        for (k, v) in doc.as_mapping_mut().unwrap() {
            if k.as_str().unwrap().starts_with(matcher.key.as_str()) {
               *v = serde_yaml::Value::String("hello".to_string());
            } else {
                resolve(&mut doc[k], matcher, current_level + 1)?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let config: serde_json::Value = serde_yaml::from_reader(File::open("configs/main.ymlex.yml")?)?;
    let validation = validate_config(config.clone())?;

    if !validation.is_valid() {
        println!("Config does not follow schema. Please refer to schema.");
        println!("{:#?}", validation.errors);
        process::exit(1);
    }
    let mut matcher: models::Matcher = serde_json::from_value(config["matcher"].clone())?;
    if matcher.level.max == -1 {
        matcher.level.max = std::i8::MAX;
    }

    for doc_deserializer in
        serde_yaml::Deserializer::from_reader(File::open("examples/example.yml")?)
    {
        let mut doc = serde_yaml::Value::deserialize(doc_deserializer)?;
        resolve(&mut doc, &matcher, 0);
    }
    // println!("{:#?}", data);

    Ok(())
}
