//! Generates the json schemas in `bindings/schemas` from the rust format types.
//!
//! Normally run through `bindings/generate.sh`, which also regenerates the models derived from the
//! output. See `bindings/README.md`.
//!
//! Input and output documents use different contracts: a field which is defaulted when reading can
//! be guaranteed when writing, `statistic.times.commuting` being one such case.

use schemars::{JsonSchema, generate::SchemaSettings};
use std::error::Error;
use std::path::Path;
use vrp_cli::extensions::solve::config::Config;
use vrp_cli::pragmatic::format::FormatError;
use vrp_cli::pragmatic::format::problem::{Matrix, Problem};
use vrp_cli::pragmatic::format::solution::Solution;

fn main() -> Result<(), Box<dyn Error>> {
    // resolved against the crate, so the command is not sensitive to where it is invoked from
    let out_dir =
        std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/bindings/schemas").to_string());
    let out_dir = Path::new(&out_dir);
    std::fs::create_dir_all(out_dir)?;

    write_input::<Problem>(out_dir, "problem")?;
    write_input::<Matrix>(out_dir, "matrix")?;
    write_input::<Config>(out_dir, "config")?;

    write_output::<Solution>(out_dir, "solution")?;
    write_output::<FormatError>(out_dir, "error")?;

    Ok(())
}

/// Describes how a document the solver reads is deserialized.
fn write_input<T: JsonSchema>(dir: &Path, name: &str) -> Result<(), Box<dyn Error>> {
    write::<T>(dir, name, SchemaSettings::draft2020_12())
}

/// Describes how a document the solver writes is serialized.
fn write_output<T: JsonSchema>(dir: &Path, name: &str) -> Result<(), Box<dyn Error>> {
    write::<T>(dir, name, SchemaSettings::draft2020_12().for_serialize())
}

fn write<T: JsonSchema>(dir: &Path, name: &str, settings: SchemaSettings) -> Result<(), Box<dyn Error>> {
    let schema = settings.into_generator().into_root_schema_for::<T>();
    let path = dir.join(format!("{name}.schema.json"));

    // trailing newline keeps the files diff and editor friendly
    std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&schema)?))?;
    println!("{}", path.display());

    Ok(())
}
