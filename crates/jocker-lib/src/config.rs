use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ConfigFile {
    pub default: Option<ConfigDefault>,
    #[serde(default)]
    pub stacks: HashMap<String, ConfigStack>,
    pub processes: HashMap<String, ConfigProcess>,
}

impl ConfigFile {
    pub fn load(target_dir: &Path) -> Result<Option<Self>> {
        let filepath = target_dir.join("jocker.yml");
        if !filepath.exists() {
            return Ok(None);
        }
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        let res = serde_yml::from_reader(reader)?;
        Ok(res)
    }
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ConfigDefault {
    pub stack: Option<String>,
    pub process: Option<ConfigProcessDefault>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ConfigProcessDefault {
    #[serde(default)]
    pub cargo_args: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ConfigStack {
    #[serde(default)]
    pub inherits: HashSet<String>,
    #[serde(default)]
    pub processes: HashSet<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ConfigProcess {
    pub binary: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cargo_args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use schemars::schema_for;

    use super::*;

    #[test]
    #[ignore = "Temporary thing to generate JsonSchema"]
    fn generate_json_schema() {
        let schema = schema_for!(ConfigFile);
        File::create(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("schema.json"),
        )
        .unwrap()
        .write_all(serde_json::to_string_pretty(&schema).unwrap().as_bytes())
        .unwrap();
    }
}
