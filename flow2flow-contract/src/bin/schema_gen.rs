use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    out_dir.push("schemas");
    let written = flow2flow_contract::write_schema_files(&out_dir)?;
    for path in written {
        println!("wrote schema: {}", path.display());
    }
    Ok(())
}
