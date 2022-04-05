use anyhow::Result;
use cmd_lib::spawn_with_output;
use std::{fs::File, io::{self, Cursor}};
use valico::json_schema::{self, ValidationState};

pub fn overlaying_config(
    default: &serde_yaml::Value,
    overlay: &mut serde_yaml::Value,
) -> Result<()> {
    if let Some(map) = default.as_mapping() {
        for (k, v) in map {
            if let None = overlay.get(k) {
                overlay[k] = v.clone();
            }
            if serde_yaml::Value::is_mapping(&overlay[k]) {
                overlaying_config(v, &mut overlay[k])?
            }
        }
    }
    Ok(())
}

pub fn validate_config(config: serde_json::Value) -> Result<ValidationState> {
    let json_schema: serde_json::Value =
        serde_json::from_reader(File::open("configs/ymlex.schema.json").unwrap()).unwrap();

    let mut scope = json_schema::Scope::new();
    let schema = scope
        .compile_and_return(json_schema.clone(), false)
        .unwrap();

    Ok(schema.validate(&config))
}

// pub fn get_file_content() -> Result<String> {
//   spawn_with_output!(curl -L)

// local:
//   file
//   dir
//   stdin
// remote: s3/http
//   file
//   tar
// }

pub async fn get_file_over_http(url: &str, path: &str) -> Result<()> {
    let resp = reqwest::get(url).await?;
    let mut out = File::create(path).expect("failed to create file");
    let mut content =  Cursor::new(resp.bytes().await?);
    io::copy(&mut content, &mut out).expect("failed to copy content");
    Ok(())
}
