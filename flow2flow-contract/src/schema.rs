use std::fs;
use std::path::{Path, PathBuf};

use schemars::{schema::RootSchema, schema_for};

use crate::{CallSpec, FlowSpec, NodeDef};

#[derive(Debug, Clone)]
pub struct SchemaArtifact {
    pub file_name: &'static str,
    pub schema: RootSchema,
}

pub fn flow_spec_schema() -> RootSchema {
    schema_for!(FlowSpec)
}

pub fn node_def_schema() -> RootSchema {
    schema_for!(NodeDef)
}

pub fn call_spec_schema() -> RootSchema {
    schema_for!(CallSpec)
}

pub fn schema_artifacts() -> Vec<SchemaArtifact> {
    vec![
        SchemaArtifact { file_name: "flow_spec.schema.json", schema: flow_spec_schema() },
        SchemaArtifact { file_name: "node_def.schema.json", schema: node_def_schema() },
        SchemaArtifact { file_name: "call_spec.schema.json", schema: call_spec_schema() },
    ]
}

pub fn write_schema_files(dir: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let mut written = Vec::new();

    for artifact in schema_artifacts() {
        let path = dir.join(artifact.file_name);
        let data = serde_json::to_string_pretty(&artifact.schema).map_err(std::io::Error::other)?;
        fs::write(&path, format!("{data}\n"))?;
        written.push(path);
    }

    Ok(written)
}
